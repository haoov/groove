use crate::changes::aligned;
use crate::words::between;

/// What each side marks, as the text under the marks.
fn marked(before: &str, after: &str) -> (Vec<String>, Vec<String>) {
    let (old, new) = between(before, after);
    (cut(before, &old), cut(after, &new))
}

fn cut(line: &str, ranges: &[std::ops::Range<usize>]) -> Vec<String> {
    ranges
        .iter()
        .map(|at| line.chars().skip(at.start).take(at.len()).collect())
        .collect()
}

#[test]
fn one_word_changed_marks_that_word_on_both_sides() {
    let marks = marked("let value = one(a, b);", "let value = two(a, b);");
    assert_eq!(marks, (vec!["one".to_string()], vec!["two".to_string()]));
}

#[test]
fn a_word_added_marks_nothing_on_the_side_that_lacks_it() {
    let (old, new) = marked("fn one(a) {", "fn one(a, b) {");
    assert!(old.is_empty(), "{old:?}");
    assert_eq!(new, vec![", b".to_string()]);
}

#[test]
fn a_mark_changed_alone_does_not_carry_the_name_beside_it() {
    let (old, new) = marked("value[at]", "value.at");
    assert_eq!(old, vec!["[".to_string(), "]".to_string()]);
    assert_eq!(new, vec![".".to_string()]);
}

#[test]
fn two_lines_with_nothing_in_common_are_marked_whole() {
    assert_eq!(
        marked("alpha beta", "delta gamma"),
        (Vec::new(), Vec::new())
    );
}

#[test]
fn a_line_too_long_to_read_is_marked_whole() {
    let before = "a".repeat(500);
    let after = format!("{}b", "a".repeat(499));
    assert_eq!(marked(&before, &after), (Vec::new(), Vec::new()));
}

#[test]
fn a_changed_line_pairs_with_the_one_it_replaced() {
    let file = aligned("a.rs", "let one = 1;\nkept\n", "let two = 1;\nkept\n");
    let removed = file.rows.iter().position(|row| row.old == Some(0)).unwrap();
    let added = file.rows.iter().position(|row| row.new == Some(0)).unwrap();
    assert_eq!(file.words.get(&removed).map(Vec::len), Some(1));
    assert_eq!(file.words.get(&added).map(Vec::len), Some(1));
    assert_eq!(
        file.words.len(),
        2,
        "the line neither touched is left alone"
    );
}
