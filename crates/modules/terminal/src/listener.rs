use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use groove_exec::pty::Pty;
use groove_types::AnsiPalette;

use crate::color;
use crate::size::Size;

/// Answers the terminal's questions on the reader thread; everything else is dropped.
pub(crate) struct Listener {
    pty: Arc<Mutex<Pty>>,
    size: Arc<Size>,
    palette: AnsiPalette,
}

impl Listener {
    pub fn new(pty: Arc<Mutex<Pty>>, size: Arc<Size>, palette: AnsiPalette) -> Self {
        Self { pty, size, palette }
    }

    fn reply(&self, text: &str) {
        let _ = crate::lock(&self.pty).write(text.as_bytes());
    }
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(text) => self.reply(&text),
            Event::ColorRequest(index, format) => {
                let rgb = color::indexed(index, &self.palette);
                self.reply(&format(color::to_vte(rgb)));
            }
            Event::TextAreaSizeRequest(format) => self.reply(&format(self.size.window())),
            _ => {}
        }
    }
}
