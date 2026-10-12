//! A note's anchor.

#[test]
fn a_note_s_lines_are_counted_from_one_and_put_in_order() {
    let anchor = crate::Anchor::numbered("a.rs", 9, Some(4)).unwrap();
    assert_eq!((anchor.start_line, anchor.end_line), (3, 8));
    let one = crate::Anchor::numbered("a.rs", 0, None).unwrap();
    assert_eq!((one.start_line, one.end_line), (0, 0));
    assert!(crate::Anchor::numbered("a.rs", i64::MAX, None).is_none());
}
