//! A terminal: a reader thread feeds the grid, a writer thread owns the PTY's input.

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

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::selection::{Selection, SelectionType};
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
    palette: Arc<Mutex<AnsiPalette>>,
    held: Mutex<Option<Held>>,
}

impl Terminal {
    pub fn spawn(spec: PtySpec, palette: AnsiPalette, hooks: Hooks) -> Result<Self> {
        let size = Arc::new(Size::new(spec.cols, spec.rows));
        let spawned = groove_exec::pty::spawn(spec)?;
        let pid = spawned.pty.pid();
        let ops = writer::start(spawned.pty)?;
        let palette = Arc::new(Mutex::new(palette));
        let listener = Listener::new(ops.clone(), size.clone(), palette.clone());
        let term = Term::new(Config::default(), &size.dims(), listener);
        let term = Arc::new(Mutex::new(term));
        reader::start(spawned.reader, spawned.child, term.clone(), hooks)?;
        Ok(Self {
            term,
            ops,
            pid,
            size,
            palette,
            held: Mutex::new(None),
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
        self.apply();
        let palette = *lock(&self.palette);
        screen::snapshot(&lock(&self.term), &palette)
    }

    /// The colours the next screen and the next colour query read.
    pub fn recolor(&self, palette: AnsiPalette) {
        *lock(&self.palette) = palette;
    }

    /// Whether the program in it reads the mouse itself.
    pub fn reads_mouse(&self) -> bool {
        lock(&self.term).mode().intersects(TermMode::MOUSE_MODE)
    }

    /// A selection begun at a cell: one press, one word, or one line.
    pub fn select_from(&self, cell: (usize, usize), kind: Select) {
        *lock(&self.held) = Some(Held {
            from: cell,
            to: None,
            kind,
        });
        self.apply();
    }

    /// The selection carried to another cell.
    pub fn select_to(&self, cell: (usize, usize)) {
        if let Some(held) = lock(&self.held).as_mut() {
            held.to = Some(cell);
        }
        self.apply();
    }

    /// The selection put back on the grid, whatever the program did under it.
    fn apply(&self) {
        let held = *lock(&self.held);
        let mut term = lock(&self.term);
        term.selection = held.map(|held| {
            let side = alacritty_terminal::index::Side::Left;
            let mut one = Selection::new(held.kind.own(), point(&term, held.from), side);
            if let Some(to) = held.to {
                one.update(point(&term, to), alacritty_terminal::index::Side::Right);
            }
            one
        });
    }

    /// What is selected, or nothing at all.
    pub fn selected(&self) -> Option<String> {
        self.apply();
        lock(&self.term)
            .selection_to_string()
            .filter(|one| !one.is_empty())
    }

    pub fn select_nothing(&self) {
        *lock(&self.held) = None;
        lock(&self.term).selection = None;
    }

    /// The wheel at `cell`: to a program that reads the mouse, else the held lines scroll.
    pub fn wheel(&self, lines: i32, cell: (usize, usize)) -> Result<()> {
        if lines == 0 {
            return Ok(());
        }
        let mut term = lock(&self.term);
        let mode = *term.mode();
        if mode.intersects(TermMode::MOUSE_MODE) {
            drop(term);
            let button = match lines > 0 {
                true => WHEEL_UP,
                false => WHEEL_DOWN,
            };
            let sgr = mode.contains(TermMode::SGR_MOUSE);
            let one = report(button, cell, true, sgr);
            return self.write(&one.repeat(lines.unsigned_abs() as usize));
        }
        if !mode.contains(TermMode::ALT_SCREEN) {
            term.scroll_display(Scroll::Delta(lines));
        }
        Ok(())
    }

    /// The left button at a cell, for the program that reads the mouse to answer.
    pub fn click(&self, cell: (usize, usize), pressed: bool) -> Result<()> {
        let mode = *lock(&self.term).mode();
        if !mode.intersects(TermMode::MOUSE_MODE) {
            return Ok(());
        }
        let sgr = mode.contains(TermMode::SGR_MOUSE);
        self.write(&report(LEFT, cell, pressed, sgr))
    }

    /// The pointer moved with the button down, for a program that asked for motion.
    pub fn drag(&self, cell: (usize, usize)) -> Result<()> {
        let mode = *lock(&self.term).mode();
        if !mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION) {
            return Ok(());
        }
        let sgr = mode.contains(TermMode::SGR_MOUSE);
        self.write(&report(LEFT + DRAGGING, cell, true, sgr))
    }

    /// Text typed at the child, wrapped when it asked for bracketed paste.
    pub fn paste(&self, text: &str) -> Result<()> {
        let bracketed = lock(&self.term).mode().contains(TermMode::BRACKETED_PASTE);
        let text = text.replace('\r', "").replace('\n', "\r");
        match bracketed {
            true => self.write(format!("\x1b[200~{text}\x1b[201~").as_bytes()),
            false => self.write(text.as_bytes()),
        }
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

/// A selection of the grid: where it began, where it stands, and what it takes.
#[derive(Debug, Clone, Copy)]
struct Held {
    from: (usize, usize),
    to: Option<(usize, usize)>,
    kind: Select,
}

/// What one press begins: the cells it crosses, the word under it, or its lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    Cells,
    Word,
    Line,
}

impl Select {
    fn own(self) -> SelectionType {
        match self {
            Select::Cells => SelectionType::Simple,
            Select::Word => SelectionType::Semantic,
            Select::Line => SelectionType::Lines,
        }
    }
}

/// A cell of the grid as a point of it, the lines it holds above included.
fn point<L: EventListener>(term: &Term<L>, cell: (usize, usize)) -> Point {
    let scrolled = term.grid().display_offset() as i32;
    let row = cell.1 as i32 - scrolled;
    let col = cell.0.min(term.columns().saturating_sub(1));
    Point::new(Line(row), Column(col))
}

/// The buttons a report names.
const LEFT: u8 = 0;
const DRAGGING: u8 = 32;
const WHEEL_UP: u8 = 64;
const WHEEL_DOWN: u8 = 65;

/// One mouse report at a cell, as the program asked to be told.
fn report(button: u8, cell: (usize, usize), pressed: bool, sgr: bool) -> Vec<u8> {
    let (col, row) = (cell.0 + 1, cell.1 + 1);
    if sgr {
        let end = match pressed {
            true => 'M',
            false => 'm',
        };
        return format!("\x1b[<{button};{col};{row}{end}").into_bytes();
    }
    let button = match pressed {
        true => button,
        false => 3,
    };
    x10(button, col, row)
}

/// The older encoding: one raw byte a coordinate, which cannot go past 223.
fn x10(button: u8, col: usize, row: usize) -> Vec<u8> {
    let byte = |one: usize| 32u8.saturating_add(one.min(223) as u8);
    vec![0x1b, b'[', b'M', 32 + button, byte(col), byte(row)]
}

/// A poisoned lock still holds a usable grid.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
