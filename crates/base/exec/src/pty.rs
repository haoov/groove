//! A child on a pseudo-terminal: bytes in, bytes out, resize, terminate.

use std::io::{Read, Write};
use std::path::PathBuf;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::{Error, Result};

pub struct PtySpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub rows: u16,
    pub cols: u16,
}

/// The handle kept by the owner: write, resize, terminate.
pub struct Pty {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    pid: Option<u32>,
}

/// The child, to be waited on by whoever drains the reader.
pub struct PtyChild(Box<dyn Child + Send + Sync>);

pub struct Spawned {
    pub pty: Pty,
    pub reader: Box<dyn Read + Send>,
    pub child: PtyChild,
}

pub fn spawn(spec: PtySpec) -> Result<Spawned> {
    let pair = native_pty_system()
        .openpty(size(spec.rows, spec.cols))
        .map_err(pty_error)?;
    let child = pair
        .slave
        .spawn_command(command(&spec))
        .map_err(pty_error)?;
    let pid = child.process_id();
    let writer = pair.master.take_writer().map_err(pty_error)?;
    let reader = pair.master.try_clone_reader().map_err(pty_error)?;
    Ok(Spawned {
        pty: Pty {
            writer,
            master: pair.master,
            pid,
        },
        reader,
        child: PtyChild(child),
    })
}

impl Pty {
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write_all(bytes)?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master.resize(size(rows, cols)).map_err(pty_error)
    }

    /// Asks the child to end with SIGTERM.
    pub fn terminate(&self) -> Result<()> {
        let Some(pid) = self.pid else { return Ok(()) };
        let pid = nix::unistd::Pid::from_raw(pid as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM)
            .map_err(|e| Error::Pty(e.to_string()))
    }
}

impl PtyChild {
    /// Blocks until the child exits and returns its exit code.
    pub fn wait(mut self) -> Result<u32> {
        let status = self.0.wait()?;
        Ok(status.exit_code())
    }
}

fn command(spec: &PtySpec) -> CommandBuilder {
    let mut command = CommandBuilder::new(&spec.program);
    command.args(&spec.args);
    command.cwd(&spec.cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    for key in ["TMUX", "TMUX_PANE", "STY"] {
        command.env_remove(key);
    }
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    command
}

fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn pty_error(e: anyhow::Error) -> Error {
    Error::Pty(e.to_string())
}
