//! A note's Conventional Comments label, bold or plain, and its decorations.

use super::*;

#[test]
fn a_header_shows_each_decoration_and_reads_a_bold_label() {
    let one = note(1, "agent", "**issue (agent, blocking):** this leaks");
    let drawn = in_editor(&noted(vec![one]));
    for text in ["issue", "agent", "blocking", "this leaks"] {
        assert!(drawn.iter().any(|one| one == text), "{text}: {drawn:?}");
    }
    assert!(!drawn.iter().any(|one| one.contains("**")), "{drawn:?}");
}
