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

#[test]
fn a_whole_run_beats_the_same_letters_split() {
    let query = "factories/keycloak";
    let whole = crate::sought("factories/keycloak.py", query).expect("a match");
    let split = crate::sought("factories/test_keycloak.py", query).expect("a match");
    assert!(
        whole.score > split.score,
        "{} vs {}",
        whole.score,
        split.score
    );
    assert_eq!(whole.at, (0..18).collect::<Vec<_>>());
    let named = crate::sought("factories/test_keycloak.py", query)
        .unwrap()
        .at;
    assert_eq!(
        &named[10..],
        (15..23).collect::<Vec<_>>(),
        "keycloak lands on its word"
    );
}

#[test]
fn a_word_start_beats_the_middle_of_a_word() {
    let start = crate::sought("src/layout.rs", "lay").unwrap();
    let middle = crate::sought("src/play.rs", "lay").unwrap();
    assert!(start.score > middle.score);
    assert!(crate::sought("src/play.rs", "zz").is_none());
}
