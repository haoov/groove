//! What a character belongs to, for picking out a word.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Word,
    Space,
    Mark,
}

pub fn class(c: char) -> Class {
    match c {
        _ if c.is_alphanumeric() || c == '_' => Class::Word,
        _ if c.is_whitespace() => Class::Space,
        _ => Class::Mark,
    }
}
