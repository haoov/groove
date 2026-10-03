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
    input: Option<Vec<u8>>,
}

impl Run {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            input: None,
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

    /// What the child reads on its standard input, which then closes.
    pub fn input(mut self, input: impl Into<Vec<u8>>) -> Self {
        self.input = Some(input.into());
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
        match tokio::time::timeout(self.timeout, self.spawned()).await {
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

    async fn spawned(&self) -> std::io::Result<Output> {
        let mut command = self.command();
        let Some(input) = &self.input else {
            return command.output().await;
        };
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn()?;
        let stdin = child.stdin.take();
        let fed = async move {
            if let Some(mut stdin) = stdin {
                tokio::io::AsyncWriteExt::write_all(&mut stdin, input).await?;
            }
            Ok::<(), std::io::Error>(())
        };
        let (fed, output) = tokio::join!(fed, child.wait_with_output());
        fed?;
        output
    }

    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.program);
        if let Some(path) = crate::login::adopted() {
            command.env("PATH", path);
        }
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
