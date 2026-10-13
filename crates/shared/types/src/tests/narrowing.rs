//! A search through text.

#[test]
fn a_search_ignores_case_and_answers_in_characters_of_the_line_itself() {
    use crate::occurrences;
    assert_eq!(occurrences("Fix fix FIX", "fix"), [0..3, 4..7, 8..11]);
    assert_eq!(occurrences("İstanbul é Straße", "STRASSE"), []);
    assert_eq!(occurrences("İstanbul é Straße", "straße"), vec![11..17]);
    assert_eq!(occurrences("aaaa", "aa"), [0..2, 2..4]);
    assert!(occurrences("text", "").is_empty());
}
