use groove_types::Capture;

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
