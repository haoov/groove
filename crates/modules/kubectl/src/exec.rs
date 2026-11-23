//! A command run in a pod: `kubectl exec` as an argument list, with no terminal and no input, bounded.

use std::time::Duration;

use groove_exec::run::Run;
use groove_types::Error;

use crate::Answer;
use crate::run::{LIMIT, ran_error};

/// How long a command runs when the agent names no bound, and the longest it may name.
pub const RUNS: Duration = Duration::from_secs(60);
pub const LONGEST: Duration = Duration::from_secs(300);

/// What a user's shell in a pod runs: bash where the container has it, sh otherwise.
pub const SHELL: [&str; 3] = [
    "sh",
    "-c",
    "command -v bash >/dev/null && exec bash || exec sh",
];

/// One command in one pod's container, the container the pod's first when none is named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exec {
    pub context: String,
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub command: Vec<String>,
    pub timeout: Duration,
}

impl Exec {
    /// The arguments kubectl runs it with; refused when it names nothing to run.
    pub fn argv(&self) -> Result<Vec<String>, Error> {
        self.built(false)
    }

    /// The same, run on a terminal with the keys going to it.
    pub fn terminal(&self) -> Result<Vec<String>, Error> {
        self.built(true)
    }

    fn built(&self, tty: bool) -> Result<Vec<String>, Error> {
        if self.command.is_empty() {
            return Err(Error::invalid("kube_exec needs a command to run"));
        }
        let bad = |one: &&String| one.is_empty() || one.starts_with('-') || one.contains('/');
        let mut named = [&self.pod, &self.namespace].into_iter();
        if named.any(|one| bad(&one)) || self.container.iter().any(|one| bad(&one)) {
            return Err(Error::invalid(
                "a pod or a container is named, not a flag or a path",
            ));
        }
        let mut argv = ["--context", &self.context, "--namespace", &self.namespace]
            .map(str::to_string)
            .to_vec();
        argv.extend(["exec".to_string(), self.pod.clone()]);
        if tty {
            argv.extend(["--stdin".to_string(), "--tty".to_string()]);
        }
        if let Some(container) = &self.container {
            argv.extend(["--container".to_string(), container.clone()]);
        }
        argv.push("--".to_string());
        argv.extend(self.command.iter().cloned());
        Ok(argv)
    }

    /// `psql -c 'select 1'`: the command as a shell would read it, what does not print escaped.
    pub fn shown(&self) -> String {
        let words: Vec<String> = self.command.iter().map(|one| quoted(one)).collect();
        words.join(" ")
    }
}

/// One word bare, single-quoted, or `$'…'` when it holds a character that does not print.
fn quoted(one: &str) -> String {
    let plain = |c: char| c.is_alphanumeric() || "-_./=:,@%+".contains(c);
    if !one.is_empty() && one.chars().all(plain) {
        return one.to_string();
    }
    if !one.chars().any(hidden) {
        return format!("'{}'", one.replace('\'', r"'\''"));
    }
    let escaped: String = one
        .chars()
        .map(|c| match c {
            '\\' => r"\\".to_string(),
            '\'' => r"\'".to_string(),
            '\n' => r"\n".to_string(),
            '\t' => r"\t".to_string(),
            c if hidden(c) => format!("\\u{:04X}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("$'{escaped}'")
}

/// A control character, or one that changes how the text around it reads: a bidi mark, a zero width.
fn hidden(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}')
        || matches!(c, '\u{2060}'..='\u{2069}' | '\u{FEFF}')
}

/// `exec` run with `kubectl`: its output, then what it said on stderr, failed when it exits non-zero.
pub async fn exec(kubectl: &str, exec: &Exec) -> Result<Answer, Error> {
    let ran = Run::new(kubectl).args(exec.argv()?).timeout(exec.timeout);
    let ran = ran.capped(LIMIT).await.map_err(|e| match e {
        groove_exec::Error::TimedOut { .. } => {
            let still = "; kubectl stopped, the command may still run in the pod";
            Error::new(groove_types::ErrorKind::Io, format!("{e}{still}"))
        }
        e => ran_error(e),
    })?;
    let mut text = String::from_utf8_lossy(&ran.stdout).into_owned();
    if ran.cut {
        text.push_str(&format!("\n… cut at {} KiB", LIMIT / 1024));
    }
    if !ran.stderr.trim().is_empty() {
        text.push_str(&format!("\nstderr:\n{}", ran.stderr.trim_end()));
    }
    Ok(Answer {
        text,
        failed: !ran.success && !ran.cut,
    })
}
