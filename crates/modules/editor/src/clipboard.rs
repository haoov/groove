//! What the editing surface copies through.

use std::sync::{Arc, Mutex, PoisonError};

use groove_types::{Error, ErrorKind, Result};

pub trait Clipboard: Send + Sync {
    fn read(&self) -> Option<String>;
    fn write(&self, text: &str) -> Result<()>;
}

/// The desktop's clipboard, or one of our own when it cannot be opened.
pub fn clipboard() -> Arc<dyn Clipboard> {
    match System::open() {
        Ok(system) => Arc::new(system),
        Err(_) => Arc::new(Memory::default()),
    }
}

/// The clipboard the rest of the desktop shares. Keep it: on Wayland and X11 the
/// window that copied serves the text to whoever pastes it.
pub struct System {
    held: Mutex<arboard::Clipboard>,
}

impl System {
    pub fn open() -> Result<Self> {
        let held = arboard::Clipboard::new().map_err(|e| failed("open the clipboard", &e))?;
        Ok(Self {
            held: Mutex::new(held),
        })
    }
}

impl Clipboard for System {
    fn read(&self) -> Option<String> {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_text()
            .ok()
    }

    fn write(&self, text: &str) -> Result<()> {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .set_text(text)
            .map_err(|e| failed("write the clipboard", &e))
    }
}

/// A clipboard of its own, which nothing else can see.
#[derive(Default)]
pub struct Memory {
    held: Mutex<Option<String>>,
}

impl Clipboard for Memory {
    fn read(&self) -> Option<String> {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn write(&self, text: &str) -> Result<()> {
        *self.held.lock().unwrap_or_else(PoisonError::into_inner) = Some(text.to_string());
        Ok(())
    }
}

fn failed(what: &str, e: &arboard::Error) -> Error {
    Error::new(ErrorKind::Io, format!("cannot {what}: {e}"))
}
