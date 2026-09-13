//! A terminal. Bytes from the child feed the grid on a reader thread; the owner
//! writes, resizes and takes a `Screen` when it draws.

mod color;
mod listener;
mod reader;
mod screen;
mod size;

#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use alacritty_terminal::term::{Config, Term};
use groove_exec::pty::Pty;
pub use groove_exec::pty::PtySpec;
pub use groove_exec::{Error, Result};
pub use groove_types::{AnsiPalette, Screen, ScreenCell};

use listener::Listener;
use size::Size;

/// What the owner hears from the reader thread.
pub struct Hooks {
    /// The grid changed. Called once per chunk of bytes.
    pub on_damage: Box<dyn Fn() + Send + Sync>,
    /// The child exited with this code. Called once, after the last bytes.
    pub on_exit: Box<dyn FnOnce(u32) + Send>,
}

pub struct Terminal {
    term: Arc<Mutex<Term<Listener>>>,
    pty: Arc<Mutex<Pty>>,
    size: Arc<Size>,
    palette: AnsiPalette,
}

impl Terminal {
    pub fn spawn(spec: PtySpec, palette: AnsiPalette, hooks: Hooks) -> Result<Self> {
        let size = Arc::new(Size::new(spec.cols, spec.rows));
        let spawned = groove_exec::pty::spawn(spec)?;
        let pty = Arc::new(Mutex::new(spawned.pty));
        let listener = Listener::new(pty.clone(), size.clone(), palette);
        let term = Term::new(Config::default(), &size.dims(), listener);
        let term = Arc::new(Mutex::new(term));
        reader::start(spawned.reader, spawned.child, term.clone(), hooks)?;
        Ok(Self {
            term,
            pty,
            size,
            palette,
        })
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        lock(&self.pty).write(bytes)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        if self.size.get() == (cols, rows) {
            return Ok(());
        }
        self.size.set(cols, rows);
        lock(&self.term).resize(self.size.dims());
        lock(&self.pty).resize(rows, cols)
    }

    pub fn size(&self) -> (u16, u16) {
        self.size.get()
    }

    pub fn screen(&self) -> Screen {
        screen::snapshot(&lock(&self.term), &self.palette)
    }

    /// Asks the child to end. The exit hook fires when it has.
    pub fn terminate(&self) -> Result<()> {
        lock(&self.pty).terminate()
    }
}

impl fmt::Debug for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (cols, rows) = self.size.get();
        write!(f, "Terminal({cols}x{rows})")
    }
}

/// A poisoned lock still holds a usable grid.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
