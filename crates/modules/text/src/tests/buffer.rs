use groove_types::{Caret, Edit, Motion};

use crate::{Buffer, Document};

fn buffer(text: &str) -> Buffer {
    Buffer::new(Document::new("src/lib.rs", text))
}

fn text(buffer: &Buffer) -> String {
    buffer.document().text()
}

fn edit(buffer: &mut Buffer, edits: &[Edit]) {
    for one in edits {
        buffer.edit(one);
    }
}

fn typed(text: &str) -> Vec<Edit> {
    text.chars().map(|c| Edit::Insert(c.to_string())).collect()
}

#[test]
fn typing_lands_where_the_caret_is_and_takes_it_along() {
    let mut buffer = buffer("fn one() {}\n");
    edit(&mut buffer, &[Edit::Move(Motion::LineEnd)]);
    edit(&mut buffer, &typed(" // note"));
    assert_eq!(text(&buffer), "fn one() {} // note\n");
    assert_eq!(buffer.caret(), Caret::new(0, 19));
    assert!(buffer.dirty());
}

#[test]
fn a_newline_splits_the_line_and_opens_the_next() {
    let mut buffer = buffer("fn one() {}\n");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 9))), Edit::Newline],
    );
    assert_eq!(text(&buffer), "fn one() \n{}\n");
    assert_eq!(buffer.caret(), Caret::new(1, 0));
}

#[test]
fn backspace_at_the_start_of_a_line_joins_it_to_the_one_above() {
    let mut buffer = buffer("one\ntwo\n");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(1, 0))), Edit::Backspace],
    );
    assert_eq!(text(&buffer), "onetwo\n");
    assert_eq!(buffer.caret(), Caret::new(0, 3));
}

#[test]
fn nothing_is_erased_past_either_end() {
    let mut buffer = buffer("one");
    edit(&mut buffer, &[Edit::Backspace]);
    assert_eq!(text(&buffer), "one", "nothing before the first character");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(9, 9))), Edit::Delete],
    );
    assert_eq!(text(&buffer), "one", "nor after the last");
    assert!(!buffer.dirty(), "and neither counts as a change");
}

#[test]
fn delete_at_the_end_of_the_last_line_takes_the_break_that_ends_it() {
    let mut buffer = buffer("one\n");
    edit(&mut buffer, &[Edit::Move(Motion::LineEnd), Edit::Delete]);
    assert_eq!(text(&buffer), "one", "the file no longer ends open");
}

#[test]
fn a_typing_run_undoes_in_one_step() {
    let mut buffer = buffer("fn one() {}\n");
    edit(&mut buffer, &[Edit::Move(Motion::LineEnd)]);
    edit(&mut buffer, &typed(" // note"));
    edit(&mut buffer, &[Edit::Undo]);
    assert_eq!(text(&buffer), "fn one() {}\n", "the whole run went");
    edit(&mut buffer, &[Edit::Redo]);
    assert_eq!(text(&buffer), "fn one() {} // note\n");
}

#[test]
fn a_move_ends_the_run_so_the_next_undo_stops_there() {
    let mut buffer = buffer("one\n");
    edit(&mut buffer, &[Edit::Move(Motion::LineEnd)]);
    edit(&mut buffer, &typed("two"));
    edit(&mut buffer, &[Edit::Move(Motion::LineStart)]);
    edit(&mut buffer, &typed("zero"));
    edit(&mut buffer, &[Edit::Undo]);
    assert_eq!(text(&buffer), "onetwo\n", "only the second run went");
    edit(&mut buffer, &[Edit::Undo]);
    assert_eq!(text(&buffer), "one\n");
    edit(&mut buffer, &[Edit::Undo]);
    assert_eq!(text(&buffer), "one\n", "and there is nothing left to undo");
}

#[test]
fn a_new_change_forgets_what_was_undone() {
    let mut buffer = buffer("one\n");
    edit(&mut buffer, &[Edit::Move(Motion::LineEnd)]);
    edit(&mut buffer, &typed("two"));
    edit(&mut buffer, &[Edit::Undo]);
    edit(&mut buffer, &typed("three"));
    edit(&mut buffer, &[Edit::Redo]);
    assert_eq!(text(&buffer), "onethree\n", "the redo had nothing to say");
}

#[test]
fn the_caret_stops_at_the_end_of_the_line_it_moves_to() {
    let mut buffer = buffer("a long line\nshort\n");
    edit(
        &mut buffer,
        &[
            Edit::Move(Motion::To(Caret::new(0, 11))),
            Edit::Move(Motion::Down),
        ],
    );
    assert_eq!(buffer.caret(), Caret::new(1, 5), "clamped to what is there");
}

#[test]
fn the_colours_follow_an_edit_before_they_are_read_again() {
    let mut buffer = buffer("fn one() {}\n");
    let before = buffer.document().spans(0).len();
    assert!(before > 0, "the fixture is highlighted");
    edit(&mut buffer, &[Edit::Move(Motion::To(Caret::new(0, 0)))]);
    edit(&mut buffer, &typed("  "));
    let shifted = buffer.document().spans(0);
    assert!(
        shifted.iter().all(|span| span.range.start >= 2),
        "every span moved along: {shifted:?}"
    );
    let revision = buffer.revision();
    buffer.reparse(revision);
    assert_eq!(buffer.document().spans(0).len(), before, "read again");
}

#[test]
fn a_stale_recolour_is_dropped() {
    let mut buffer = buffer("fn one() {}\n");
    let revision = buffer.revision();
    edit(&mut buffer, &typed("x"));
    buffer.reparse(revision);
    assert_ne!(
        buffer.revision(),
        revision,
        "the buffer moved on, so the colours are not installed"
    );
}

#[test]
fn a_shifted_motion_grows_the_range_the_caret_owns() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[
            Edit::Extend(Motion::Right),
            Edit::Extend(Motion::Right),
            Edit::Extend(Motion::Right),
        ],
    );
    assert_eq!(buffer.selected(), "one");
    let one = buffer.selections()[0];
    assert_eq!(one.anchor, Caret::new(0, 0));
    assert_eq!(one.head, Caret::new(0, 3));
}

#[test]
fn typing_over_a_selection_puts_it_out() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[Edit::Extend(Motion::Right), Edit::Extend(Motion::Right)],
    );
    edit(&mut buffer, &typed("O"));
    assert_eq!(text(&buffer), "Oe two\n");
    assert_eq!(buffer.caret(), Caret::new(0, 1));
    assert!(
        buffer.selections()[0].is_empty(),
        "and nothing is left held"
    );
}

#[test]
fn backspace_takes_the_selection_rather_than_a_character() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[
            Edit::Move(Motion::To(Caret::new(0, 4))),
            Edit::Extend(Motion::LineEnd),
            Edit::Backspace,
        ],
    );
    assert_eq!(text(&buffer), "one \n");
}

#[test]
fn a_plain_motion_lets_the_selection_go() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[Edit::Extend(Motion::Right), Edit::Move(Motion::Right)],
    );
    assert_eq!(buffer.selected(), "", "nothing is held");
    assert_eq!(buffer.caret(), Caret::new(0, 2));
}

#[test]
fn select_all_holds_the_whole_document() {
    let mut buffer = buffer("one\ntwo\n");
    edit(&mut buffer, &[Edit::SelectAll]);
    assert_eq!(buffer.selected(), "one\ntwo\n");
    edit(&mut buffer, &typed("x"));
    assert_eq!(text(&buffer), "x", "and typing replaces it");
}

#[test]
fn undoing_a_selection_that_was_typed_over_brings_it_back() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[
            Edit::Extend(Motion::Right),
            Edit::Extend(Motion::Right),
            Edit::Extend(Motion::Right),
        ],
    );
    edit(&mut buffer, &typed("ONE"));
    assert_eq!(text(&buffer), "ONE two\n");
    edit(&mut buffer, &[Edit::Undo]);
    assert_eq!(text(&buffer), "one two\n", "the words came back");
}

#[test]
fn what_a_selection_covers_on_a_line_is_the_part_of_it_that_is_there() {
    let mut buffer = buffer("one\ntwo\nthree\n");
    edit(
        &mut buffer,
        &[
            Edit::Move(Motion::To(Caret::new(0, 1))),
            Edit::Extend(Motion::To(Caret::new(2, 2))),
        ],
    );
    let one = buffer.selections()[0];
    assert_eq!(one.on(0, 3), Some((1, 3, true)), "from the caret, and on");
    assert_eq!(one.on(1, 3), Some((0, 3, true)), "the whole line");
    assert_eq!(one.on(2, 5), Some((0, 2, false)), "up to the caret");
    assert_eq!(one.on(3, 0), None, "and nothing past it");
}

#[test]
fn an_indent_step_is_what_the_language_writes() {
    let mut rust = Buffer::new(Document::new("src/lib.rs", "fn one() {}\n"));
    rust.edit(&Edit::Indent);
    assert_eq!(text(&rust), "    fn one() {}\n", "rust writes spaces");

    let mut go = Buffer::new(Document::new("main.go", "func one() {}\n"));
    go.edit(&Edit::Indent);
    assert_eq!(text(&go), "\tfunc one() {}\n", "go writes a tab");

    let mut yaml = Buffer::new(Document::new("ci.yaml", "jobs:\n"));
    yaml.edit(&Edit::Indent);
    assert_eq!(text(&yaml), "  jobs:\n", "yaml writes two spaces");

    let mut plain = Buffer::new(Document::new("notes.txt", "one\n"));
    plain.edit(&Edit::Indent);
    assert_eq!(
        text(&plain),
        "    one\n",
        "and a file with no grammar, four"
    );
}

#[test]
fn a_word_is_taken_with_its_own_kind_of_characters() {
    let mut buffer = buffer("let value = one();\n");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 5))), Edit::SelectWord],
    );
    assert_eq!(buffer.selected(), "value", "from inside the word");

    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 3))), Edit::SelectWord],
    );
    assert_eq!(buffer.selected(), " ", "the space between two words");

    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 15))), Edit::SelectWord],
    );
    assert_eq!(buffer.selected(), "();", "a run of marks");
}

#[test]
fn a_word_is_taken_from_either_of_its_ends() {
    let mut buffer = buffer("one two\n");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 4))), Edit::SelectWord],
    );
    assert_eq!(buffer.selected(), "two", "from its first character");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(0, 7))), Edit::SelectWord],
    );
    assert_eq!(buffer.selected(), "two", "and from its last");
}

#[test]
fn a_line_is_taken_with_the_break_that_ends_it() {
    let mut buffer = buffer("one\ntwo\nthree\n");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(1, 2))), Edit::SelectLine],
    );
    assert_eq!(buffer.selected(), "two\n");
    edit(&mut buffer, &[Edit::Backspace]);
    assert_eq!(
        text(&buffer),
        "one\nthree\n",
        "so taking it out takes the row away"
    );
}

#[test]
fn the_last_line_is_taken_without_a_break_it_does_not_have() {
    let mut buffer = buffer("one\ntwo");
    edit(
        &mut buffer,
        &[Edit::Move(Motion::To(Caret::new(1, 1))), Edit::SelectLine],
    );
    assert_eq!(buffer.selected(), "two");
}
