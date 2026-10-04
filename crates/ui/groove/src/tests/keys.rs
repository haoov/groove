//! A winit key as the ui reads it, on a layout with dead keys.

use groove_ui::input::Key;
use winit::keyboard::Key as WinitKey;

use crate::keys::key_of;

#[test]
fn a_dead_key_under_a_chord_reads_as_the_key_it_sits_on() {
    assert_eq!(
        key_of(&WinitKey::Dead(Some('´')), true),
        Some(Key::Char('\''))
    );
    assert_eq!(
        key_of(&WinitKey::Dead(Some('¨')), true),
        Some(Key::Char('"'))
    );
    assert_eq!(
        key_of(&WinitKey::Dead(Some('^')), true),
        Some(Key::Char('^'))
    );
}

#[test]
fn a_dead_key_alone_waits_for_the_next_key() {
    assert_eq!(key_of(&WinitKey::Dead(Some('´')), false), None);
}
