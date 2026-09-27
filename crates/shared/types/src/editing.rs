/// Where the caret sits in a document: a line, and a column in characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Caret {
    pub line: usize,
    pub column: usize,
}

impl Caret {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// What one indent step writes, and how wide it reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Indent {
    /// A tab character, read as this many columns.
    Tab(usize),
    Spaces(usize),
}

impl Indent {
    /// What a tab key puts in.
    pub fn text(&self) -> String {
        match self {
            Indent::Tab(_) => "\t".to_string(),
            Indent::Spaces(width) => " ".repeat(*width),
        }
    }

    /// How many columns it stands.
    pub fn width(&self) -> usize {
        match self {
            Indent::Tab(width) | Indent::Spaces(width) => *width,
        }
    }
}

/// One caret's range: where it began, and where the caret is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Selection {
    pub anchor: Caret,
    pub head: Caret,
}

impl Selection {
    pub fn at(caret: Caret) -> Self {
        Self {
            anchor: caret,
            head: caret,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The two ends in reading order.
    pub fn ends(&self) -> (Caret, Caret) {
        match self.anchor <= self.head {
            true => (self.anchor, self.head),
            false => (self.head, self.anchor),
        }
    }

    /// The columns this selection covers on `line`, and whether it runs past the end.
    pub fn on(&self, line: usize, chars: usize) -> Option<(usize, usize, bool)> {
        let (from, to) = self.ends();
        if self.is_empty() || line < from.line || line > to.line {
            return None;
        }
        let start = match line == from.line {
            true => from.column,
            false => 0,
        };
        let through = line < to.line;
        let end = match through {
            true => chars,
            false => to.column,
        };
        Some((start.min(chars), end.min(chars), through))
    }
}

/// Where the caret goes next.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    /// Where a click landed. The buffer clamps it to a place that exists.
    To(Caret),
}

/// What the keyboard asks of the open buffer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Edit {
    Insert(String),
    /// One indent step, as the language writes it.
    Indent,
    /// Moves the caret and leaves the anchor.
    Extend(Motion),
    SelectAll,
    /// What a double click takes: the word, the spaces or the marks under the caret.
    SelectWord,
    /// What a triple click takes: the line and the break that ends it.
    SelectLine,
    Newline,
    Backspace,
    Delete,
    Move(Motion),
    Undo,
    Redo,
}
