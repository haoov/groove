//! A child piped for as long as it lives: its stdin, stdout and stderr held open, killed when dropped.

use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

use crate::{Error, Result};

pub struct Piped {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env: Vec<(String, String)>,
    unset: Vec<String>,
}

/// A running child. Dropping it kills the child.
pub struct Process {
    child: tokio::process::Child,
    pid: Option<u32>,
    pipes: Option<Pipes>,
}

/// The three pipes of a child, taken once from its `Process`.
pub struct Pipes {
    pub stdin: ChildStdin,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

impl Piped {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            unset: Vec::new(),
        }
    }

    crate::described!();

    /// A variable the child does not inherit.
    pub fn unset(mut self, key: impl Into<String>) -> Self {
        self.unset.push(key.into());
        self
    }

    /// Starts the child with its three pipes open.
    pub fn start(self) -> Result<Process> {
        let mut command = tokio::process::Command::new(&self.program);
        for key in &self.unset {
            command.env_remove(key);
        }
        crate::run::prepared(&mut command, &self.args, &self.env, self.cwd.as_deref());
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|source| Error::Spawn {
            program: self.describe(),
            source,
        })?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(Error::Io(std::io::Error::other("a pipe did not open")));
        };
        Ok(Process {
            pid: child.id(),
            child,
            pipes: Some(Pipes {
                stdin,
                stdout,
                stderr,
            }),
        })
    }
}

impl Process {
    /// The child's pipes; `None` once taken.
    pub fn pipes(&mut self) -> Option<Pipes> {
        self.pipes.take()
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Waits for the child to end; its exit code, or `None` when a signal ended it.
    pub async fn wait(&mut self) -> Result<Option<i32>> {
        Ok(self.child.wait().await?.code())
    }

    /// Asks the child to end with SIGTERM. Never blocks.
    pub fn terminate(&self) -> Result<()> {
        let Some(pid) = self.pid else {
            return Ok(());
        };
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM)
            .map_err(|e| Error::Io(e.into()))
    }
}
