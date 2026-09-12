//! A child run to completion: captured output, a timeout, killed when dropped.

use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use crate::{Error, Result, redact};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

pub struct Run {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
    timeout: Duration,
}

impl Run {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The command line with credentials redacted, for messages and logs.
    pub fn describe(&self) -> String {
        redact(&format!("{} {}", self.program, self.args.join(" ")))
    }

    /// Runs and returns the raw output, whatever the exit status.
    pub async fn output(self) -> Result<Output> {
        let program = self.describe();
        let mut command = self.command();
        match tokio::time::timeout(self.timeout, command.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(source)) => Err(Error::Spawn { program, source }),
            Err(_) => Err(Error::TimedOut {
                program,
                after: self.timeout,
            }),
        }
    }

    /// Runs and returns stdout; a non-zero exit is an error carrying stderr.
    pub async fn text(self) -> Result<String> {
        let program = self.describe();
        let output = self.output().await?;
        if !output.status.success() {
            return Err(Error::Failed {
                program,
                stderr: redact(String::from_utf8_lossy(&output.stderr).trim()),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.program);
        command
            .args(&self.args)
            .envs(self.env.iter().cloned())
            .kill_on_drop(true);
        if let Some(dir) = &self.cwd {
            command.current_dir(dir);
        }
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Run {
        Run::new("sh").args(["-c", script])
    }

    #[tokio::test]
    async fn output_carries_status_stdout_and_stderr() {
        let out = sh("echo out; echo err >&2; exit 3").output().await.unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(out.stdout, b"out\n");
        assert_eq!(out.stderr, b"err\n");
    }

    #[tokio::test]
    async fn text_returns_stdout_on_success() {
        assert_eq!(sh("printf hello").text().await.unwrap(), "hello");
    }

    #[tokio::test]
    async fn text_reports_redacted_stderr_on_failure() {
        let err = sh("echo 'https://user:secret@host/p' >&2; exit 1")
            .text()
            .await
            .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("<redacted>@host/p"), "{message}");
        assert!(!message.contains("secret"), "{message}");
    }

    #[tokio::test]
    async fn a_slow_child_times_out() {
        let err = sh("sleep 5")
            .timeout(Duration::from_millis(50))
            .output()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::TimedOut { .. }), "{err}");
    }

    #[tokio::test]
    async fn a_missing_program_fails_to_spawn() {
        let err = Run::new("groove-no-such-program")
            .output()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Spawn { .. }), "{err}");
    }

    #[tokio::test]
    async fn cwd_and_env_reach_the_child() {
        let out = sh("pwd; printf %s \"$GROOVE_X\"")
            .cwd("/")
            .env("GROOVE_X", "y")
            .text()
            .await
            .unwrap();
        assert_eq!(out, "/\ny");
    }
}
