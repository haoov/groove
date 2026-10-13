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
    assert_eq!(said.decoration, Some("blocking"));
    assert_eq!(said.rest, "it leaks\nsee line 4");
    let bare = crate::labelled("Nitpick: a space").expect("any case");
    assert_eq!(
        (bare.label, bare.decoration, bare.rest),
        ("nitpick", None, "a space")
    );
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
