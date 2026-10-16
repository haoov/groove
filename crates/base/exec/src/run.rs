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
    bare: bool,
}

/// A run's output read up to a limit, and whether the limit cut it.
#[derive(Debug)]
pub struct Capped {
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub success: bool,
    pub cut: bool,
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
            bare: false,
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

    /// The child starts with no environment but the one given.
    pub fn bare(mut self) -> Self {
        self.bare = true;
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
        match tokio::time::timeout(self.timeout, self.spawned(&program)).await {
            Ok(ran) => ran,
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

    /// Runs and reads at most `limit` bytes of stdout; a child that writes more is killed.
    pub async fn capped(self, limit: usize) -> Result<Capped> {
        let program = self.describe();
        match tokio::time::timeout(self.timeout, self.read_capped(&program, limit)).await {
            Ok(ran) => ran,
            Err(_) => Err(Error::TimedOut {
                program,
                after: self.timeout,
            }),
        }
    }

    async fn read_capped(&self, program: &str, limit: usize) -> Result<Capped> {
        use tokio::io::AsyncReadExt;
        let mut command = self.command();
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().map_err(|source| Error::Spawn {
            program: program.to_string(),
            source,
        })?;
        let (stdin, input) = (child.stdin.take(), self.input.clone().unwrap_or_default());
        let fed = async move {
            if let Some(mut stdin) = stdin {
                let _ = tokio::io::AsyncWriteExt::write_all(&mut stdin, &input).await;
            }
        };
        let (out, err) = (child.stdout.take(), child.stderr.take());
        let read_out = async move {
            let mut read = Vec::new();
            if let Some(out) = out {
                out.take(limit as u64 + 1).read_to_end(&mut read).await?;
            }
            Ok::<_, std::io::Error>(read)
        };
        let read_err = async move {
            let mut read = Vec::new();
            if let Some(err) = err {
                err.take(limit as u64).read_to_end(&mut read).await?;
            }
            Ok::<_, std::io::Error>(read)
        };
        let ((), stdout, stderr) = tokio::join!(fed, read_out, read_err);
        let (mut stdout, stderr) = (stdout?, stderr?);
        let cut = stdout.len() > limit;
        if cut {
            stdout.truncate(limit);
            let _ = child.kill().await;
        }
        let success = child.wait().await?.success();
        let stderr = redact(String::from_utf8_lossy(&stderr).trim());
        Ok(Capped {
            stdout,
            stderr,
            success: success && !cut,
            cut,
        })
    }

    /// The child run to its end; a failure to start it is `Spawn`, one after is `Io`.
    async fn spawned(&self, program: &str) -> Result<Output> {
        let started = |source| Error::Spawn {
            program: program.to_string(),
            source,
        };
        let mut command = self.command();
        let Some(input) = &self.input else {
            return command.output().await.map_err(started);
        };
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = command.spawn().map_err(started)?;
        let stdin = child.stdin.take();
        let fed = async move {
            if let Some(mut stdin) = stdin {
                tokio::io::AsyncWriteExt::write_all(&mut stdin, input).await?;
            }
            Ok::<(), std::io::Error>(())
        };
        let (fed, output) = tokio::join!(fed, child.wait_with_output());
        match fed {
            Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => Err(e.into()),
            _ => Ok(output?),
        }
    }

    fn command(&self) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(&self.program);
        if self.bare {
            command.env_clear();
        }
        prepared(&mut command, &self.args, &self.env, self.cwd.as_deref());
        command
    }
}

/// Sets the login `PATH`, `args`, `env` and `cwd`, and kill on drop.
pub(crate) fn prepared(
    command: &mut tokio::process::Command,
    args: &[String],
    env: &[(String, String)],
    cwd: Option<&Path>,
) {
    if let Some(path) = crate::login::adopted() {
        command.env("PATH", path);
    }
    command
        .args(args)
        .envs(env.iter().cloned())
        .kill_on_drop(true);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
}
