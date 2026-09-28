use groove_types::{Capture, Caret, Highlight};

use crate::Document;

/// The captures of one line, with the text each one covers.
fn spans(doc: &Document, at: usize) -> Vec<(Capture, String)> {
    let line = doc.line(at).expect("a line").to_string();
    doc.spans(at)
        .into_iter()
        .map(|span| (span.capture, line[span.range].to_string()))
        .collect()
}

#[test]
fn rust_colours_its_keywords_strings_and_comments() {
    let text = "// what it does\nfn one() -> &'static str {\n    \"two\"\n}\n";
    let doc = Document::new("a.rs", text);
    assert!(doc.is_highlighted());
    assert_eq!(
        spans(&doc, 0),
        [(Capture::Comment, "// what it does".into())]
    );
    let head = spans(&doc, 1);
    assert!(
        head.contains(&(Capture::Keyword, "fn".to_string())),
        "{head:?}"
    );
    assert!(
        head.contains(&(Capture::Function, "one".to_string())),
        "{head:?}"
    );
    assert_eq!(spans(&doc, 2), [(Capture::String, "\"two\"".into())]);
}

#[test]
fn a_span_crossing_lines_is_cut_at_each_one() {
    let text = "/* one\n   two */\nfn three() {}\n";
    let doc = Document::new("a.rs", text);
    assert_eq!(spans(&doc, 0), [(Capture::Comment, "/* one".into())]);
    assert_eq!(spans(&doc, 1), [(Capture::Comment, "   two */".into())]);
    assert!(
        spans(&doc, 2)
            .iter()
            .all(|(kind, _)| *kind != Capture::Comment)
    );
}

#[test]
fn every_language_we_ship_colours_something() {
    let samples = [
        ("a.rs", "fn one() {}\n"),
        ("a.yml", "name: groove\non: push\n"),
        ("a.sh", "if true; then echo one; fi\n"),
        ("a.md", "# Title\n\ntext\n"),
        ("a.py", "def one():\n    return 1\n"),
        ("a.go", "func one() int { return 1 }\n"),
    ];
    for (path, text) in samples {
        let doc = Document::new(path, text);
        assert!(doc.is_highlighted(), "{path} has no spans");
    }
}

#[test]
fn a_yaml_key_and_a_markdown_heading_carry_their_own_meaning() {
    let yaml = Document::new("ci.yml", "name: groove\njobs: 2\n");
    let keys = spans(&yaml, 0);
    assert!(
        keys.contains(&(Capture::Attribute, "name".into())),
        "{keys:?}"
    );
    assert!(spans(&yaml, 1).contains(&(Capture::Number, "2".into())));

    let md = Document::new("README.md", "# Title\n\n    code\n");
    let title = spans(&md, 0);
    assert!(
        title.iter().any(|(kind, _)| *kind == Capture::Title),
        "{title:?}"
    );
}

/// Every line's captures, for comparing two reads of the same text.
fn whole(doc: &Document) -> Vec<Vec<(Capture, String)>> {
    (0..doc.lines()).map(|at| spans(doc, at)).collect()
}

#[test]
fn an_edited_tree_colours_like_a_fresh_one() {
    let source = "fn one() -> usize {\n    let it = \"text\";\n    1\n}\n";
    for typed in ["value", "\"", "}", "/* "] {
        let mut doc = Document::new("src/lib.rs", source);
        let at = doc.char_of(Caret::new(1, 13));
        doc.insert(at, typed);
        doc.remove(at - 4..at - 1);
        doc.reparse();
        let fresh = Document::new("src/lib.rs", &doc.text());
        assert_eq!(whole(&doc), whole(&fresh), "after typing {typed:?}");
    }
}

#[test]
fn a_window_colours_a_comment_that_starts_above_it() {
    let doc = Document::new("src/lib.rs", "/* one\ntwo\nthree\nfour */\nfn five() {}\n");
    let window = doc.colours(2..3);
    assert_eq!(
        window.of(2),
        [Highlight {
            range: 0..5,
            capture: Capture::Comment
        }],
        "the comment opened two lines above the window"
    );
    assert!(window.of(1).is_empty(), "a line the window left out");
    assert!(window.of(4).is_empty());
}

#[test]
fn the_scopes_of_a_line_stack_from_the_outside_in() {
    let source = "mod one {\n    impl Two {\n        fn three() {\n            let four = 4;\n        }\n    }\n}\n";
    let doc = Document::new("src/lib.rs", source);
    assert_eq!(doc.scopes(3), [0, 1, 2], "mod, impl, fn");
    assert_eq!(
        doc.scopes(2),
        [0, 1],
        "the line a scope opens on is not its own scope"
    );
    assert_eq!(doc.scopes(0), Vec::<usize>::new());
}

#[test]
fn a_yaml_key_path_and_a_markdown_heading_are_scopes() {
    let yaml = Document::new(
        "ci.yml",
        "spec:\n  containers:\n    resources:\n      cpu: 1\n",
    );
    assert_eq!(yaml.scopes(3), [0, 1, 2]);
    let md = Document::new("README.md", "# One\n\n## Two\n\ntext\n");
    assert_eq!(md.scopes(4), [0, 2], "the headings above it");
}

#[test]
fn a_parse_read_before_later_edits_lands_moved_along_them() {
    let source = "fn one() -> usize {\n    1\n}\n";
    let mut doc = Document::new("src/lib.rs", source);
    doc.insert(0, "// head\n");
    let read = doc.clone();
    doc.insert(0, "\n\n\n");
    doc.insert(doc.char_of(Caret::new(5, 4)), "let two = \"2\"; ");
    assert!(
        doc.install(read.settled()),
        "a parse of this document lands"
    );
    let fresh = Document::new("src/lib.rs", &doc.text());
    let (now, want) = (whole(&doc), whole(&fresh));
    assert_eq!(
        now[3..5],
        want[3..5],
        "what it read, coloured where it now stands"
    );
    doc.reparse();
    assert_eq!(whole(&doc), want, "and the next parse takes the rest");
}

#[test]
fn a_parse_of_another_document_does_not_land() {
    let mut doc = Document::new("src/lib.rs", "fn one() {}\n");
    let other = Document::new("src/lib.rs", "fn one() {}\n");
    assert!(!doc.install(other.settled()));
}
