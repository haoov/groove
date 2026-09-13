use std::sync::mpsc::{Receiver, Sender, channel};

use groove_exec::pty::Pty;

use crate::Result;

/// What the writer thread does to the PTY, in the order asked.
pub(crate) enum Op {
    Write(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

/// The one thread that touches the PTY's write side. Dropping every `Sender` ends it.
pub(crate) fn start(pty: Pty) -> Result<Sender<Op>> {
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("terminal-writer".into())
        .spawn(move || run(pty, rx))?;
    Ok(tx)
}

fn run(mut pty: Pty, ops: Receiver<Op>) {
    while let Ok(op) = ops.recv() {
        let _ = match op {
            Op::Write(bytes) => pty.write(&bytes),
            Op::Resize { cols, rows } => pty.resize(rows, cols),
        };
    }
}
