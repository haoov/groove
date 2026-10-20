//! What the editing surface copies through.

use std::sync::{Arc, Mutex};

use groove_types::{Error, ErrorKind, Result};

/// The desktop's clipboard, or one of our own; every read and write runs off the caller's thread.
#[derive(Clone)]
pub struct Clipboard {
    held: Arc<dyn Held>,
}

impl Clipboard {
    /// The desktop's clipboard, or one of our own when it cannot be opened.
    pub fn system() -> Self {
        match System::open() {
            Ok(system) => Self {
                held: Arc::new(system),
            },
            Err(_) => Self::memory(),
        }
    }

    /// A clipboard of its own, which nothing else can see.
    pub fn memory() -> Self {
        Self {
            held: Arc::new(Memory::default()),
        }
    }

    pub async fn read(&self) -> Option<String> {
        let held = self.held.clone();
        tokio::task::spawn_blocking(move || held.read())
            .await
            .ok()
            .flatten()
    }

    pub async fn write(&self, text: &str) -> Result<()> {
        let (held, text) = (self.held.clone(), text.to_string());
        match tokio::task::spawn_blocking(move || held.write(&text)).await {
            Ok(written) => written,
            Err(e) => Err(Error::new(ErrorKind::Io, e.to_string())),
        }
    }
}

trait Held: Send + Sync {
    fn read(&self) -> Option<String>;
    fn write(&self, text: &str) -> Result<()>;
}

/// The desktop's clipboard. Keep it alive: on Wayland and X11 the window that copied serves the paste.
struct System {
    held: Mutex<arboard::Clipboard>,
}

impl System {
    fn open() -> Result<Self> {
        let held = arboard::Clipboard::new().map_err(|e| failed("open the clipboard", &e))?;
        Ok(Self {
            held: Mutex::new(held),
        })
    }
}

impl Held for System {
    fn read(&self) -> Option<String> {
        groove_sync::lock(&self.held).get_text().ok()
    }

    fn write(&self, text: &str) -> Result<()> {
        groove_sync::lock(&self.held)
            .set_text(text)
            .map_err(|e| failed("write the clipboard", &e))
    }
}

#[derive(Default)]
struct Memory {
    held: Mutex<Option<String>>,
}

impl Held for Memory {
    fn read(&self) -> Option<String> {
        groove_sync::lock(&self.held).clone()
    }

    fn write(&self, text: &str) -> Result<()> {
        *groove_sync::lock(&self.held) = Some(text.to_string());
        Ok(())
    }
}

fn failed(what: &str, e: &arboard::Error) -> Error {
    Error::new(ErrorKind::Io, format!("cannot {what}: {e}"))
}
