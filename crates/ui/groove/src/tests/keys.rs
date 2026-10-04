//! A winit key as the ui reads it, on a layout with dead keys.

use groove_ui::input::Key;
use winit::keyboard::{Key as WinitKey, NativeKey};

use crate::keys::key_of;

#[test]
fn a_dead_key_under_a_chord_reads_as_the_key_it_sits_on() {
    let dead = |sym| WinitKey::Unidentified(NativeKey::Xkb(sym));
    assert_eq!(key_of(&dead(0xFE51), true), Some(Key::Char('\'')));
    assert_eq!(key_of(&dead(0xFE50), true), Some(Key::Char('`')));
}

#[test]
fn a_dead_key_alone_waits_for_the_next_key() {
    let acute = WinitKey::Unidentified(NativeKey::Xkb(0xFE51));
    assert_eq!(key_of(&acute, false), None);
}
