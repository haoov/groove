//! The input method: what it composes, what it types, and where its window opens.

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::Ime;

use super::App;
use groove_ui::input::Input;

impl App {
    pub(super) fn ime(&mut self, event: Ime) {
        let input = match event {
            Ime::Preedit(text, _) => Input::Preedit(Some(text)),
            Ime::Commit(text) => Input::Commit(text),
            Ime::Disabled => Input::Preedit(None),
            Ime::Enabled => return,
        };
        self.input(input);
    }

    /// The input method's window at the caret that has the keyboard, once it has moved.
    pub(super) fn place_ime(&mut self) {
        let (Some(window), Some(caret)) = (&self.window, self.hits.caret()) else {
            return;
        };
        if self.ime_at == Some(caret) {
            return;
        }
        self.ime_at = Some(caret);
        let at = PhysicalPosition::new(caret.x, caret.y);
        window.set_ime_cursor_area(at, PhysicalSize::new(caret.w, caret.h));
    }
}
