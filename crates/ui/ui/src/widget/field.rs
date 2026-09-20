//! A one-line field: what is typed into it, and where its caret sits.

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Field {
    text: String,
    /// The caret, in characters before it.
    at: usize,
}

impl Field {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.at = 0;
    }

    /// Takes this text whole, with the caret at its end.
    pub fn set(&mut self, text: &str) {
        self.text = text.to_string();
        self.at = self.chars();
    }

    pub fn insert(&mut self, c: char) {
        self.text.insert(self.byte(self.at), c);
        self.at += 1;
    }

    /// Takes the character before the caret.
    pub fn backspace(&mut self) {
        if self.at == 0 {
            return;
        }
        self.at -= 1;
        self.text.remove(self.byte(self.at));
    }

    /// Takes the character the caret sits before.
    pub fn delete(&mut self) {
        if self.at < self.chars() {
            self.text.remove(self.byte(self.at));
        }
    }

    pub fn left(&mut self) {
        self.at = self.at.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.at = (self.at + 1).min(self.chars());
    }

    pub fn home(&mut self) {
        self.at = 0;
    }

    pub fn end(&mut self) {
        self.at = self.chars();
    }

    /// The text with a bar drawn where the caret is.
    pub fn shown(&self) -> String {
        let at = self.byte(self.at);
        format!("{}\u{2502}{}", &self.text[..at], &self.text[at..])
    }

    fn chars(&self) -> usize {
        self.text.chars().count()
    }

    fn byte(&self, at: usize) -> usize {
        self.text
            .char_indices()
            .nth(at)
            .map(|(byte, _)| byte)
            .unwrap_or(self.text.len())
    }
}
