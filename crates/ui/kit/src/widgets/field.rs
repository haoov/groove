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

    /// `text` at the caret, on the one line a field holds.
    pub fn paste(&mut self, text: &str) {
        for c in Self::one_line(text).chars() {
            self.insert(c);
        }
    }

    /// `text` at the caret, its line breaks kept.
    pub fn paste_lines(&mut self, text: &str) {
        for c in text.trim().chars().filter(|one| *one != '\r') {
            self.insert(c);
        }
    }

    /// The text a one-line field can hold: every break a space, and none at its ends.
    pub fn one_line(text: &str) -> String {
        text.replace(['\n', '\r'], " ").trim().to_string()
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
        self.composed(None)
    }

    /// The same, with what the input method holds in front of the bar.
    pub fn composed(&self, preedit: Option<&str>) -> String {
        format!(
            "{}{}\u{2502}{}",
            self.before(),
            preedit.unwrap_or(""),
            self.after()
        )
    }

    pub fn before(&self) -> &str {
        &self.text[..self.byte(self.at)]
    }

    pub fn after(&self) -> &str {
        &self.text[self.byte(self.at)..]
    }

    /// The same, a bullet for each character.
    pub fn masked(&self) -> String {
        let (before, after) = (self.at, self.chars() - self.at);
        format!("{}\u{2502}{}", "•".repeat(before), "•".repeat(after))
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
