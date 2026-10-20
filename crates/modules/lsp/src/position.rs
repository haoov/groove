//! Columns in characters, as the caret counts them, against the units the server counts.

/// The unit a server counts a column in, agreed at `initialize`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    /// The encoding a server names; UTF-16 when it names none, as the protocol says.
    pub(crate) fn named(name: Option<&str>) -> Self {
        match name {
            Some("utf-8") => Self::Utf8,
            Some("utf-32") => Self::Utf32,
            _ => Self::Utf16,
        }
    }

    fn width(self, c: char) -> usize {
        match self {
            Self::Utf8 => c.len_utf8(),
            Self::Utf16 => c.len_utf16(),
            Self::Utf32 => 1,
        }
    }

    /// The units before character `column` of `line`.
    pub(crate) fn units(self, line: &str, column: usize) -> usize {
        line.chars().take(column).map(|c| self.width(c)).sum()
    }

    /// The character at `units` of `line`; past the end, the units past it count as characters.
    pub(crate) fn column(self, line: &str, units: usize) -> usize {
        let mut seen = 0;
        for (column, c) in line.chars().enumerate() {
            if seen >= units {
                return column;
            }
            seen += self.width(c);
        }
        line.chars().count() + units.saturating_sub(seen)
    }
}
