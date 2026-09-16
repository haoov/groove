use std::ops::Range;

/// What a piece of code is, as a grammar reads it. Colour is the ui's to choose.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capture {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Constant,
    Attribute,
    Title,
    Literal,
    Link,
}

/// One run of a line, in bytes from the line's start.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Highlight {
    pub range: Range<usize>,
    pub capture: Capture,
}
