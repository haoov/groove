//! What the colour cache keeps, and when it gives it up.

use groove_controllers::workspace_service::Document;

use crate::painted::Painted;

fn doc() -> Document {
    let text: String = (0..200)
        .map(|at| format!("fn name_{at}() -> usize {{ {at} }}\n"))
        .collect();
    Document::new("src/lib.rs", &text)
}

#[test]
fn the_same_rows_are_read_once() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, 0), 10..50);
    let again = painted.of(&doc, "src/lib.rs", false, (1, 0), 10..50);
    assert!(std::rc::Rc::ptr_eq(&first, &again), "read again");
    assert!(!first.of(20).is_empty(), "the line is coloured");
}

#[test]
fn a_small_scroll_keeps_what_it_has_and_a_long_one_does_not() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, 0), 60..100);
    let near = painted.of(&doc, "src/lib.rs", false, (1, 0), 70..110);
    assert!(
        std::rc::Rc::ptr_eq(&first, &near),
        "still inside what it read"
    );
    let far = painted.of(&doc, "src/lib.rs", false, (1, 0), 160..200);
    assert!(!std::rc::Rc::ptr_eq(&first, &far));
}

#[test]
fn a_document_that_moved_is_read_again() {
    let (painted, doc) = (Painted::default(), doc());
    let first = painted.of(&doc, "src/lib.rs", false, (1, 0), 10..50);
    let typed = painted.of(&doc, "src/lib.rs", false, (1, 1), 10..50);
    assert!(!std::rc::Rc::ptr_eq(&first, &typed), "the buffer moved on");
    let loaded = painted.of(&doc, "src/lib.rs", false, (2, 1), 10..50);
    assert!(
        !std::rc::Rc::ptr_eq(&typed, &loaded),
        "the file was read again"
    );
}
