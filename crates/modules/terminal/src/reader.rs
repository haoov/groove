use std::io::{ErrorKind, Read};
use std::sync::{Arc, Mutex};

use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::Processor;
use groove_exec::pty::PtyChild;

use crate::listener::Listener;
use crate::{Hooks, Result, lock};

const BUFFER: usize = 16 * 1024;

/// The reader thread: bytes to the grid, one damage call per chunk, the exit code last.
pub(crate) fn start(
    mut reader: Box<dyn Read + Send>,
    child: PtyChild,
    term: Arc<Mutex<Term<Listener>>>,
    hooks: Hooks,
) -> Result<()> {
    std::thread::Builder::new()
        .name("terminal-reader".into())
        .spawn(move || {
            let mut parser: Processor = Processor::new();
            let mut buf = [0u8; BUFFER];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        parser.advance(&mut *lock(&term), &buf[..n]);
                        (hooks.on_damage)();
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            let code = child.wait().unwrap_or(1);
            (hooks.on_exit)(code);
        })?;
    Ok(())
}
