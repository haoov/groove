use groove_types::{Capture, Highlight};

use crate::{column_of, display_of, expand, spans_of};

const WIDTH: usize = 4;

#[test]
fn a_tab_runs_out_to_the_next_stop_not_a_fixed_width() {
    assert_eq!(expand("\tone", WIDTH), "    one");
    assert_eq!(expand("a\tb", WIDTH), "a   b", "three columns from one");
    assert_eq!(expand("abc\td", WIDTH), "abc d", "one column from three");
    assert_eq!(expand("abcd\te", WIDTH), "abcd    e", "a whole step");
    assert_eq!(expand("\t\tx", WIDTH), "        x");
}

#[test]
fn a_line_with_no_tab_is_left_as_it_is() {
    assert_eq!(expand("fn one() {}", WIDTH), "fn one() {}");
    assert_eq!(display_of("fn one()", 4, WIDTH), 4);
    assert_eq!(column_of("fn one()", 4, WIDTH), 4);
}

#[test]
fn a_column_reads_the_same_both_ways() {
    let line = "\tif x {\n";
    for column in 0..line.chars().count() {
        let display = display_of(line, column, WIDTH);
        assert_eq!(
            column_of(line, display, WIDTH),
            column,
            "column {column} at display {display}"
        );
    }
}

#[test]
fn a_click_inside_a_tab_lands_on_the_tab() {
    let line = "\tone";
    assert_eq!(column_of(line, 0, WIDTH), 0, "before it");
    for display in 1..=3 {
        assert_eq!(column_of(line, display, WIDTH), 1, "inside it: {display}");
    }
    assert_eq!(column_of(line, 4, WIDTH), 1, "the character after it");
    assert_eq!(column_of(line, 5, WIDTH), 2);
}

#[test]
fn a_column_past_the_line_is_its_end() {
    assert_eq!(column_of("\tx", 99, WIDTH), 2);
    assert_eq!(display_of("\tx", 99, WIDTH), 5);
}

#[test]
fn the_colours_move_with_the_text_they_are_over() {
    let line = "\tlet x = 1;";
    let spans = [
        Highlight {
            range: 1..4,
            capture: Capture::Keyword,
        },
        Highlight {
            range: 9..10,
            capture: Capture::Number,
        },
    ];
    let moved = spans_of(&spans, line, WIDTH);
    let shown = expand(line, WIDTH);
    assert_eq!(&shown[moved[0].range.clone()], "let");
    assert_eq!(&shown[moved[1].range.clone()], "1");
}

#[test]
fn a_tab_wide_as_eight_reads_as_eight() {
    assert_eq!(expand("\tx", 8), "        x");
    assert_eq!(display_of("\tx", 1, 8), 8);
}
