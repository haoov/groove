use std::process::Output;

use groove_exec::redact;
use groove_exec::run::Run;

use crate::{Error, Git, Result};

impl Git {
    /// `git <args>` in this directory, English messages, never a prompt.
    fn run(&self, args: &[&str]) -> Run {
        Run::new("git")
            .args(args.iter().copied())
            .cwd(&self.dir)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes")
    }

    /// The raw output, whatever the exit status.
    pub(crate) async fn output(&self, args: &[&str]) -> Result<Output> {
        Ok(self.run(args).output().await?)
    }

    /// The same, with `input` on the child's standard input.
    pub(crate) async fn fed(&self, args: &[&str], input: String) -> Result<Output> {
        Ok(self.run(args).input(input).output().await?)
    }

    /// Stdout of a command that succeeded; a failure is an error carrying stderr.
    pub(crate) async fn text(&self, args: &[&str]) -> Result<String> {
        let output = self.output(args).await?;
        if !output.status.success() {
            return Err(Error::Failed {
                command: describe(args),
                stderr: redact(String::from_utf8_lossy(&output.stderr).trim()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// One trimmed line of stdout.
    pub(crate) async fn line(&self, args: &[&str]) -> Result<String> {
        Ok(self.text(args).await?.trim().to_string())
    }

    /// Whether the command exits zero.
    pub(crate) async fn succeeds(&self, args: &[&str]) -> Result<bool> {
        Ok(self.output(args).await?.status.success())
    }
}

pub(crate) fn describe(args: &[&str]) -> String {
    redact(&format!("git {}", args.join(" ")))
}
