use std::sync::Arc;
use std::sync::mpsc::Sender;

use alacritty_terminal::event::{Event, EventListener};
use groove_types::AnsiPalette;

use crate::color;
use crate::size::Size;
use crate::writer::Op;

/// Answers the terminal's questions on the reader thread; everything else is dropped.
pub(crate) struct Listener {
    ops: Sender<Op>,
    size: Arc<Size>,
    palette: AnsiPalette,
}

impl Listener {
    pub fn new(ops: Sender<Op>, size: Arc<Size>, palette: AnsiPalette) -> Self {
        Self { ops, size, palette }
    }

    fn reply(&self, text: &str) {
        let _ = self.ops.send(Op::Write(text.as_bytes().to_vec()));
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
