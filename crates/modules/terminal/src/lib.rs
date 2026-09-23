//! A terminal. A reader thread feeds the child's bytes to the grid, a writer thread
//! owns the PTY's input side; the owner takes a `Screen` when it draws.

mod color;
mod listener;
mod reader;
mod screen;
mod size;
mod writer;

#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};

use alacritty_terminal::grid::Scroll;
use alacritty_terminal::term::{Config, Term, TermMode};
pub use groove_exec::pty::PtySpec;
pub use groove_exec::{Error, Result};
pub use groove_types::{AnsiPalette, Screen, ScreenCell};

use listener::Listener;
use size::Size;
use writer::Op;

/// What the owner hears from the reader thread.
pub struct Hooks {
    /// The grid changed. Called once per chunk of bytes.
    pub on_damage: Box<dyn Fn() + Send + Sync>,
    /// The child exited with this code. Called once, after the last bytes.
    pub on_exit: Box<dyn FnOnce(u32) + Send>,
}

pub struct Terminal {
    term: Arc<Mutex<Term<Listener>>>,
    ops: Sender<Op>,
    pid: Option<u32>,
    size: Arc<Size>,
    palette: AnsiPalette,
}

impl Terminal {
    pub fn spawn(spec: PtySpec, palette: AnsiPalette, hooks: Hooks) -> Result<Self> {
        let size = Arc::new(Size::new(spec.cols, spec.rows));
        let spawned = groove_exec::pty::spawn(spec)?;
        let pid = spawned.pty.pid();
        let ops = writer::start(spawned.pty)?;
        let listener = Listener::new(ops.clone(), size.clone(), palette);
        let term = Term::new(Config::default(), &size.dims(), listener);
        let term = Arc::new(Mutex::new(term));
        reader::start(spawned.reader, spawned.child, term.clone(), hooks)?;
        Ok(Self {
            term,
            ops,
            pid,
            size,
            palette,
        })
    }

    /// Queues bytes for the PTY. Returns at once; the writer thread delivers them in order.
    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        self.send(Op::Write(bytes.to_vec()))
    }

    /// The grid now, the PTY through the writer thread.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        if self.size.get() == (cols, rows) {
            return Ok(());
        }
        self.size.set(cols, rows);
        lock(&self.term).resize(self.size.dims());
        self.send(Op::Resize { cols, rows })
    }

    pub fn size(&self) -> (u16, u16) {
        self.size.get()
    }

    pub fn screen(&self) -> Screen {
        screen::snapshot(&lock(&self.term), &self.palette)
    }

    /// The wheel over the grid. A program that reads the mouse is sent the wheel
    /// itself, at `cell`; anything else scrolls the lines the terminal holds.
    pub fn wheel(&self, lines: i32, cell: (usize, usize)) -> Result<()> {
        if lines == 0 {
            return Ok(());
        }
        let mut term = lock(&self.term);
        let mode = *term.mode();
        if mode.intersects(TermMode::MOUSE_MODE) {
            drop(term);
            return self.write(&reported(lines, cell, mode.contains(TermMode::SGR_MOUSE)));
        }
        if !mode.contains(TermMode::ALT_SCREEN) {
            term.scroll_display(Scroll::Delta(lines));
        }
        Ok(())
    }

    /// Asks the child to end with SIGTERM, even when its input is blocked. The exit hook fires when it has.
    pub fn terminate(&self) -> Result<()> {
        match self.pid {
            Some(pid) => groove_exec::pty::terminate(pid),
            None => Ok(()),
        }
    }

    fn send(&self, op: Op) -> Result<()> {
        self.ops
            .send(op)
            .map_err(|_| Error::Pty("the terminal is closed".into()))
    }
}

impl fmt::Debug for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (cols, rows) = self.size.get();
        write!(f, "Terminal({cols}x{rows})")
    }
}

/// The wheel as a program that reads the mouse takes it: one report a line.
fn reported(lines: i32, cell: (usize, usize), sgr: bool) -> Vec<u8> {
    let button = match lines > 0 {
        true => 64,
        false => 65,
    };
    let (col, row) = (cell.0 + 1, cell.1 + 1);
    let one = match sgr {
        true => format!("\x1b[<{button};{col};{row}M"),
        false => x10(button, col, row),
    };
    one.repeat(lines.unsigned_abs() as usize).into_bytes()
}

/// The older encoding, whose one byte a coordinate cannot take past 223.
fn x10(button: u8, col: usize, row: usize) -> String {
    let byte = |one: usize| char::from(32u8.saturating_add(one.min(223) as u8));
    format!(
        "\x1b[M{}{}{}",
        char::from(32 + button),
        byte(col),
        byte(row)
    )
}

/// A poisoned lock still holds a usable grid.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
