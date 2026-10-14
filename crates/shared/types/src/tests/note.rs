//! A note's anchor.

#[test]
fn a_note_s_lines_are_counted_from_one_and_put_in_order() {
    let anchor = crate::Anchor::numbered("a.rs", 9, Some(4)).unwrap();
    assert_eq!((anchor.start_line, anchor.end_line), (3, 8));
    let one = crate::Anchor::numbered("a.rs", 0, None).unwrap();
    assert_eq!((one.start_line, one.end_line), (0, 0));
    assert!(crate::Anchor::numbered("a.rs", i64::MAX, None).is_none());
}

#[test]
fn a_conventional_comment_gives_its_label_its_decoration_and_the_rest() {
    let said = crate::labelled("issue (blocking): it leaks\nsee line 4").expect("a label");
    assert_eq!(said.label, "issue");
    assert_eq!(said.decorations, ["blocking"]);
    assert_eq!(said.rest, "it leaks\nsee line 4");
    let bare = crate::labelled("Nitpick: a space").expect("any case");
    assert_eq!((bare.label, bare.rest), ("nitpick", "a space"));
    assert!(bare.decorations.is_empty());
}

#[test]
fn words_that_only_look_like_a_label_stay_words() {
    assert_eq!(
        crate::labelled("note to self: none"),
        None,
        "two words are no label"
    );
    assert_eq!(crate::labelled("fixed it"), None, "no colon");
    assert_eq!(crate::labelled("see: https://x"), None, "not a label");
    assert_eq!(
        crate::labelled("issue (blocking: x"),
        None,
        "an open decoration"
    );
}

#[test]
fn a_bold_label_reads_as_a_plain_one() {
    let said = crate::labelled("**Praise**: clean split").expect("bold");
    assert_eq!((said.label, said.rest), ("praise", "clean split"));
    let inside = crate::labelled("**suggestion (non-blocking):** inline it").expect("colon inside");
    assert_eq!(inside.label, "suggestion");
    assert_eq!(inside.decorations, ["non-blocking"]);
    assert_eq!(inside.rest, "inline it");
}

#[test]
fn several_decorations_read_apart_and_the_agent_s_is_added_once() {
    let said = crate::labelled("issue (agent, blocking): it leaks").expect("a label");
    assert_eq!(said.decorations, ["agent", "blocking"]);
    assert_eq!(
        crate::decorated("issue (blocking): it leaks", "agent"),
        "issue (agent, blocking): it leaks"
    );
    assert_eq!(
        crate::decorated("issue (agent): x", "agent"),
        "issue (agent): x"
    );
    assert_eq!(
        crate::decorated("it leaks", "agent"),
        "note (agent): it leaks"
    );
}
