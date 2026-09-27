//! What a one-line field does with a keystroke.

use crate::widgets::Field;

fn typed(text: &str) -> Field {
    let mut field = Field::default();
    for c in text.chars() {
        field.insert(c);
    }
    field
}

#[test]
fn the_caret_moves_through_what_was_typed() {
    let mut field = typed("src/lib");
    assert_eq!(field.shown(), "src/lib\u{2502}", "it types at the end");

    field.left();
    field.left();
    field.insert('X');
    assert_eq!(field.text(), "src/lXib", "and puts a character where it is");
    assert_eq!(field.shown(), "src/lX\u{2502}ib");

    field.home();
    field.insert('.');
    assert_eq!(field.text(), ".src/lXib");
    field.end();
    field.insert('!');
    assert_eq!(field.text(), ".src/lXib!");
}

#[test]
fn backspace_takes_what_is_behind_and_delete_what_is_ahead() {
    let mut field = typed("abc");
    field.left();
    field.backspace();
    assert_eq!(field.text(), "ac", "b went, the caret stayed before c");
    field.delete();
    assert_eq!(field.text(), "a");
    field.delete();
    assert_eq!(field.text(), "a", "nothing ahead to take");
    field.backspace();
    assert!(field.is_empty());
    field.backspace();
    assert!(field.is_empty(), "nothing behind either");
}

#[test]
fn a_caret_never_lands_inside_a_character() {
    let mut field = typed("héllo");
    field.home();
    field.right();
    field.insert('-');
    assert_eq!(field.text(), "h-éllo", "counted in characters, not bytes");
    field.end();
    field.backspace();
    assert_eq!(field.text(), "h-éll");
}
