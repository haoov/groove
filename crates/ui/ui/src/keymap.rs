//! Every action a chord runs, its default chords, and the ones the config binds instead.

mod rebind;
mod table;

use groove_types::{Chord, Config, Stroke};

use crate::input::{Key, Modifiers};
pub use rebind::{rebinds, rebound, reset};
pub use table::{Action, Spec, TABLE};

/// Where a chord is heard: from anywhere, in the focused pane's content, or in a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ring {
    App,
    Code,
    Terminal,
}

/// Each action with the chords it answers to now.
#[derive(Debug, Clone, PartialEq)]
pub struct Keymap {
    bound: Vec<(Action, Vec<Chord>)>,
}

impl Keymap {
    /// The defaults, each replaced by what the config binds to its action.
    pub fn of(config: Option<&Config>) -> Self {
        let held = config.map(|c| &c.keymap);
        let bound = TABLE.iter().map(|spec| {
            let texts: Vec<&str> = match held.and_then(|one| one.get(spec.id)) {
                Some(bound) => bound.iter().map(String::as_str).collect(),
                None => spec.defaults.to_vec(),
            };
            let chords = texts.iter().filter_map(|text| Chord::parse(text)).collect();
            (spec.action, chords)
        });
        Self {
            bound: bound.collect(),
        }
    }

    pub fn chords(&self, action: Action) -> &[Chord] {
        let found = self.bound.iter().find(|(one, _)| *one == action);
        found.map_or(&[], |(_, chords)| chords.as_slice())
    }

    /// Whether the key pressed is one of the action's chords.
    pub fn is(&self, action: Action, key: Key, mods: Modifiers) -> bool {
        let pressed = chord_of(key, mods);
        self.chords(action).contains(&pressed)
    }

    /// The action of the app's own ring the key runs, if any.
    pub fn app(&self, key: Key, mods: Modifiers) -> Option<Action> {
        let pressed = chord_of(key, mods);
        let app = TABLE.iter().filter(|spec| spec.ring == Ring::App);
        let mut held = app.map(|spec| spec.action);
        held.find(|action| self.chords(*action).contains(&pressed))
    }

    /// The first chord of an action, as the palette and Settings write it.
    pub fn label(&self, action: Action) -> Option<String> {
        self.chords(action).first().map(ToString::to_string)
    }

    /// The same, for the action a palette entry or a command id names.
    pub fn label_of(&self, id: &str) -> Option<String> {
        let spec = TABLE.iter().find(|spec| spec.id == id)?;
        self.label(spec.action)
    }
}

/// Whether the key pressed pastes, in the code or in a terminal.
pub fn pastes(config: Option<&Config>, key: Key, mods: Modifiers) -> bool {
    let keymap = Keymap::of(config);
    [Action::Paste, Action::TerminalPaste]
        .into_iter()
        .any(|action| keymap.is(action, key, mods))
}

/// The key pressed, as a chord; a letter is read in lower case.
pub fn chord_of(key: Key, mods: Modifiers) -> Chord {
    let key = match key {
        Key::Char(c) => Stroke::Char(c.to_ascii_lowercase()),
        Key::Enter => Stroke::Enter,
        Key::Tab => Stroke::Tab,
        Key::Escape => Stroke::Escape,
        Key::Backspace => Stroke::Backspace,
        Key::Delete => Stroke::Delete,
        Key::Up => Stroke::Up,
        Key::Down => Stroke::Down,
        Key::Left => Stroke::Left,
        Key::Right => Stroke::Right,
        Key::Home => Stroke::Home,
        Key::End => Stroke::End,
        Key::PageUp => Stroke::PageUp,
        Key::PageDown => Stroke::PageDown,
    };
    Chord {
        ctrl: mods.ctrl,
        alt: mods.alt,
        shift: mods.shift,
        key,
    }
}
