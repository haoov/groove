//! A key with its modifiers, as the keymap binds it and as the config file writes it.

use std::fmt;

/// A key a chord ends on; a character is the one the key types without a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stroke {
    Char(char),
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

const NAMED: [(Stroke, &str); 13] = [
    (Stroke::Enter, "enter"),
    (Stroke::Tab, "tab"),
    (Stroke::Escape, "esc"),
    (Stroke::Backspace, "backspace"),
    (Stroke::Delete, "delete"),
    (Stroke::Up, "up"),
    (Stroke::Down, "down"),
    (Stroke::Left, "left"),
    (Stroke::Right, "right"),
    (Stroke::Home, "home"),
    (Stroke::End, "end"),
    (Stroke::PageUp, "pageup"),
    (Stroke::PageDown, "pagedown"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: Stroke,
}

impl Chord {
    /// `ctrl+shift+z`, `alt+'`, `ctrl+enter`: modifiers first, the key last.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_lowercase();
        let (mods, key) = match text.strip_suffix("++") {
            Some(mods) => (mods, "+"),
            None => match text.rfind('+') {
                Some(at) if at + 1 < text.len() => (&text[..at], &text[at + 1..]),
                _ => ("", text.as_str()),
            },
        };
        let mut chord = Chord {
            ctrl: false,
            alt: false,
            shift: false,
            key: stroke(key)?,
        };
        for one in mods.split('+').filter(|one| !one.is_empty()) {
            match one {
                "ctrl" => chord.ctrl = true,
                "alt" => chord.alt = true,
                "shift" => chord.shift = true,
                _ => return None,
            }
        }
        Some(chord)
    }
}

fn stroke(key: &str) -> Option<Stroke> {
    if let Some((one, _)) = NAMED.iter().find(|(_, name)| *name == key) {
        return Some(*one);
    }
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        _ if key == "space" => Some(Stroke::Char(' ')),
        (Some(c), None) => Some(Stroke::Char(c)),
        _ => None,
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mods = [
            (self.ctrl, "ctrl+"),
            (self.alt, "alt+"),
            (self.shift, "shift+"),
        ];
        for (held, name) in mods {
            if held {
                f.write_str(name)?;
            }
        }
        match self.key {
            Stroke::Char(' ') => f.write_str("space"),
            Stroke::Char(c) => write!(f, "{c}"),
            named => {
                let found = NAMED.iter().find(|(one, _)| *one == named);
                f.write_str(found.map_or("?", |(_, name)| name))
            }
        }
    }
}
