//! A command run in a pod: `kubectl exec` as an argument list, with no terminal and no input, bounded.

use std::time::Duration;

use groove_exec::run::Run;
use groove_types::Error;

use crate::Answer;
use crate::run::{LIMIT, ran_error};

/// How long a command runs when the agent names no bound, and the longest it may name.
pub const RUNS: Duration = Duration::from_secs(60);
pub const LONGEST: Duration = Duration::from_secs(300);

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
        if self.command.is_empty() {
            return Err(Error::invalid("kube_exec needs a command to run"));
        }
        let bad = |one: &&String| one.is_empty() || one.starts_with('-');
        let mut named = [&self.pod, &self.namespace].into_iter();
        if named.any(|one| bad(&one)) || self.container.iter().any(|one| bad(&one)) {
            return Err(Error::invalid("a pod or a container is named, not a flag"));
        }
        let mut argv = ["--context", &self.context, "--namespace", &self.namespace]
            .map(str::to_string)
            .to_vec();
        argv.extend(["exec".to_string(), self.pod.clone()]);
        if let Some(container) = &self.container {
            argv.extend(["--container".to_string(), container.clone()]);
        }
        argv.push("--".to_string());
        argv.extend(self.command.iter().cloned());
        Ok(argv)
    }

    /// `psql -c 'select 1'`: the command as a shell would read it.
    pub fn shown(&self) -> String {
        let quoted = |one: &String| {
            let plain = !one.is_empty()
                && one
                    .chars()
                    .all(|c| c.is_alphanumeric() || "-_./=:,@%+".contains(c));
            match plain {
                true => one.clone(),
                false => format!("'{}'", one.replace('\'', r"'\''")),
            }
        };
        let words: Vec<String> = self.command.iter().map(quoted).collect();
        words.join(" ")
    }
}

/// `exec` run with `kubectl`: its output, then what it said on stderr, failed when it exits non-zero.
pub async fn exec(kubectl: &str, exec: &Exec) -> Result<Answer, Error> {
    let ran = Run::new(kubectl).args(exec.argv()?).timeout(exec.timeout);
    let ran = ran.capped(LIMIT).await.map_err(ran_error)?;
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
