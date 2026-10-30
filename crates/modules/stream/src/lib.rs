//! The agent run headless: its words, interrupts and answers written to it as stream-json, what it asks read back.

mod parse;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use groove_agent_launch::Launch;
use groove_exec::piped::{Piped, Pipes};
use groove_types::chat::Permission;
use groove_types::{Error, ErrorKind, Result};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// What the agent tells Groove beside its transcript.
#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    /// A tool waits on the user's leave.
    Asks(Permission),
    /// A request the agent no longer waits on.
    Withdrawn(String),
    /// A turn ended, finished or interrupted.
    Settled,
}

/// What the owner hears from the reader.
pub struct Hooks {
    pub on_heard: Box<dyn Fn(Heard) + Send + Sync>,
    /// The child exited with this code. Called once.
    pub on_exit: Box<dyn FnOnce(u32) + Send>,
}

/// One running agent. Dropping it closes its input, which ends it.
#[derive(Debug)]
pub struct Stream {
    outgoing: mpsc::UnboundedSender<Value>,
    pid: Option<u32>,
    requests: Arc<AtomicU64>,
}

impl Stream {
    /// Spawns `launch` headless, its pipes read and written on the tokio pool.
    pub async fn spawn(launch: Launch, hooks: Hooks) -> Result<Self> {
        let mut piped = Piped::new(&launch.program)
            .args(launch.args.iter().cloned())
            .cwd(&launch.cwd);
        for (key, value) in &launch.env {
            piped = piped.env(key, value);
        }
        let mut process = piped.start().map_err(|e| agent(e.to_string()))?;
        let Some(Pipes {
            stdin,
            stdout,
            stderr,
        }) = process.pipes()
        else {
            return Err(agent("the agent's pipes were taken".into()));
        };
        let pid = process.pid();
        let stream = Self::connect(stdout, stdin, hooks.on_heard);
        tokio::spawn(async move {
            let mut stderr = stderr;
            tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await
        });
        let on_exit = hooks.on_exit;
        tokio::spawn(async move {
            let code = process.wait().await.ok().flatten().unwrap_or(-1);
            on_exit(code as u32);
        });
        Ok(Self { pid, ..stream })
    }

    /// Speaks the protocol on `reader` and `writer`.
    pub fn connect<R, W>(reader: R, writer: W, on_heard: Box<dyn Fn(Heard) + Send + Sync>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (outgoing, queued) = mpsc::unbounded_channel::<Value>();
        tokio::spawn(written(writer, queued));
        tokio::spawn(read(reader, outgoing.clone(), on_heard));
        Self {
            outgoing,
            pid: None,
            requests: Arc::default(),
        }
    }

    /// The user's words, as one message.
    pub fn say(&self, text: &str) -> Result<()> {
        let message = json!({ "role": "user", "content": text });
        self.send(json!({
            "type": "user",
            "message": message,
            "parent_tool_use_id": null,
            "session_id": "",
        }))
    }

    /// The turn stopped where it stands.
    pub fn interrupt(&self) -> Result<()> {
        let id = self.requests.fetch_add(1, Ordering::Relaxed);
        self.send(json!({
            "type": "control_request",
            "request_id": format!("groove-{id}"),
            "request": { "subtype": "interrupt" },
        }))
    }

    /// The tool `id` asked for runs, on `input`.
    pub fn allow(&self, id: &str, input: Value) -> Result<()> {
        self.answer(id, json!({ "behavior": "allow", "updatedInput": input }))
    }

    /// The tool `id` asked for does not run, and the agent hears `why`.
    pub fn deny(&self, id: &str, why: &str) -> Result<()> {
        self.answer(id, json!({ "behavior": "deny", "message": why }))
    }

    /// Asks the child to end with SIGTERM. Never blocks.
    pub fn terminate(&self) -> Result<()> {
        match self.pid {
            Some(pid) => groove_exec::pty::terminate(pid).map_err(|e| agent(e.to_string())),
            None => Ok(()),
        }
    }

    fn answer(&self, id: &str, response: Value) -> Result<()> {
        self.send(parse::success(id, response))
    }

    fn send(&self, message: Value) -> Result<()> {
        self.outgoing
            .send(message)
            .map_err(|_| agent("the agent has ended".into()))
    }
}

/// Each queued message, one line each, until the queue or the pipe closes.
async fn written<W: AsyncWrite + Unpin>(mut writer: W, mut queued: mpsc::UnboundedReceiver<Value>) {
    while let Some(message) = queued.recv().await {
        let mut line = message.to_string();
        line.push('\n');
        let sent = writer.write_all(line.as_bytes()).await;
        if sent.is_err() || writer.flush().await.is_err() {
            return;
        }
    }
}

/// Each line read until the pipe closes; a request Groove does not serve is refused.
async fn read<R: AsyncRead + Unpin>(
    reader: R,
    outgoing: mpsc::UnboundedSender<Value>,
    on_heard: Box<dyn Fn(Heard) + Send + Sync>,
) {
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        match parse::line(&line) {
            Some(Ok(heard)) => on_heard(heard),
            Some(Err(refused)) => drop(outgoing.send(refused)),
            None => {}
        }
    }
}

fn agent(message: String) -> Error {
    Error::new(ErrorKind::Agent, message)
}
