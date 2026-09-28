use crate::{Chord, Stroke};

#[test]
fn a_chord_reads_its_modifiers_then_its_key_and_writes_them_back_in_one_order() {
    for text in [
        "alt+k",
        "alt+shift+'",
        "ctrl+enter",
        "ctrl+shift+z",
        "alt+,",
        "alt+shift+up",
    ] {
        let chord = Chord::parse(text).expect(text);
        assert_eq!(chord.to_string(), text);
    }
    let written = Chord::parse("Shift+Alt+K").expect("any order, any case");
    assert_eq!(written.to_string(), "alt+shift+k");
    assert_eq!(
        Chord::parse("ctrl++").map(|one| one.key),
        Some(Stroke::Char('+'))
    );
}

#[test]
fn what_is_no_chord_is_refused() {
    for text in ["", "hyper+k", "alt+enterr", "ctrl+"] {
        assert_eq!(Chord::parse(text), None, "{text}");
    }
}
