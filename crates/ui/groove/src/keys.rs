//! A winit key event as the ui's own key and modifiers.

use groove_ui::input::{Input, Key, Modifiers};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey, NativeKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

/// The modifiers as the ui reads them.
pub fn mods_of(mods: ModifiersState) -> Modifiers {
    Modifiers {
        ctrl: mods.control_key(),
        shift: mods.shift_key(),
        alt: mods.alt_key(),
    }
}

/// With ctrl or alt held, the key as it reads without any modifier.
pub fn input_of(event: &KeyEvent, mods: ModifiersState) -> Option<Input> {
    if event.state != ElementState::Pressed {
        return None;
    }
    let bare = event.key_without_modifiers();
    let chord = mods.control_key() || mods.alt_key();
    let logical = match chord {
        true => &bare,
        false => &event.logical_key,
    };
    Some(Input::Key {
        key: key_of(logical, chord)?,
        mods: mods_of(mods),
    })
}

/// The key as the ui reads it; under a chord, a dead key reads as the key it sits on.
pub fn key_of(logical: &WinitKey, chord: bool) -> Option<Key> {
    let key = match logical {
        WinitKey::Named(NamedKey::Escape) => Key::Escape,
        WinitKey::Named(NamedKey::Enter) => Key::Enter,
        WinitKey::Named(NamedKey::Tab) => Key::Tab,
        WinitKey::Named(NamedKey::Backspace) => Key::Backspace,
        WinitKey::Named(NamedKey::Delete) => Key::Delete,
        WinitKey::Named(NamedKey::ArrowUp) => Key::Up,
        WinitKey::Named(NamedKey::ArrowDown) => Key::Down,
        WinitKey::Named(NamedKey::ArrowLeft) => Key::Left,
        WinitKey::Named(NamedKey::ArrowRight) => Key::Right,
        WinitKey::Named(NamedKey::Home) => Key::Home,
        WinitKey::Named(NamedKey::End) => Key::End,
        WinitKey::Named(NamedKey::PageUp) => Key::PageUp,
        WinitKey::Named(NamedKey::PageDown) => Key::PageDown,
        WinitKey::Named(NamedKey::Space) => Key::Char(' '),
        WinitKey::Character(text) => Key::Char(text.chars().next()?),
        WinitKey::Unidentified(NativeKey::Xkb(sym)) if chord => Key::Char(dead(*sym)?),
        _ => return None,
    };
    Some(key)
}

/// The ASCII key an X dead keysym stands on.
fn dead(sym: u32) -> Option<char> {
    match sym {
        0xFE50 => Some('`'),
        0xFE51 => Some('\''),
        0xFE52 => Some('^'),
        0xFE53 => Some('~'),
        0xFE57 => Some('"'),
        _ => None,
    }
}
