use crate::Document;

#[test]
fn search_finds_every_line_and_where_in_it() {
    let doc = Document::new("a.rs", "let one = 1;\nlet two = one + one;\n");
    let found = doc.search("one");
    let places: Vec<(usize, String)> = found
        .iter()
        .map(|it| {
            let line = doc.line(it.line).expect("a line").to_string();
            (it.line, line[it.range.clone()].to_string())
        })
        .collect();
    assert_eq!(
        places,
        [
            (0, "one".to_string()),
            (1, "one".to_string()),
            (1, "one".to_string())
        ]
    );
}

#[test]
fn search_ignores_case_and_an_empty_query_finds_nothing() {
    let doc = Document::new("a.rs", "let One = 1;\n");
    assert_eq!(doc.search("one").len(), 1);
    assert_eq!(doc.search("ONE").len(), 1);
    assert!(doc.search("").is_empty());
    assert!(doc.search("three").is_empty());
}

#[test]
fn search_reads_a_document_with_no_language() {
    let doc = Document::new("Cargo.toml", "name = \"groove\"\n");
    assert_eq!(doc.search("groove").len(), 1);
}
