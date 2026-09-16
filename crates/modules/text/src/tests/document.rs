use crate::{Document, Language};

#[test]
fn a_file_name_names_its_language() {
    assert_eq!(Language::of("crates/ui/src/lib.rs"), Some(Language::Rust));
    assert_eq!(Language::of("ci.yml"), Some(Language::Yaml));
    assert_eq!(Language::of("deploy.YAML"), Some(Language::Yaml));
    assert_eq!(Language::of("scripts/run.sh"), Some(Language::Bash));
    assert_eq!(Language::of("~/.bashrc"), Some(Language::Bash));
    assert_eq!(Language::of("README.md"), Some(Language::Markdown));
    assert_eq!(Language::of("main.py"), Some(Language::Python));
    assert_eq!(Language::of("main.go"), Some(Language::Go));
    assert_eq!(Language::of("Cargo.toml"), None);
    assert_eq!(Language::of("LICENSE"), None);
}

#[test]
fn lines_are_what_a_reader_counts() {
    assert_eq!(Document::new("a.rs", "").lines(), 0, "nothing to read");
    assert_eq!(Document::new("a.rs", "\n").lines(), 1, "one empty line");
    assert_eq!(Document::new("a.rs", "one").lines(), 1);
    assert_eq!(Document::new("a.rs", "one\n").lines(), 1);
    assert_eq!(Document::new("a.rs", "one\ntwo").lines(), 2);
    assert_eq!(Document::new("a.rs", "one\ntwo\n").lines(), 2);
    assert_eq!(Document::new("a.rs", "one\n\n").lines(), 2);
}

#[test]
fn a_line_comes_back_without_its_newline() {
    let doc = Document::new("a.rs", "fn one() {}\nfn two() {}\n");
    assert_eq!(doc.line(0).as_deref(), Some("fn one() {}"));
    assert_eq!(doc.line(1).as_deref(), Some("fn two() {}"));
    assert_eq!(doc.line(2), None);
}

#[test]
fn a_document_in_a_language_we_do_not_have_keeps_its_text() {
    let doc = Document::new("Cargo.toml", "[package]\nname = \"groove\"\n");
    assert!(doc.language().is_none());
    assert!(!doc.is_highlighted());
    assert_eq!(doc.lines(), 2);
    assert_eq!(doc.line(0).as_deref(), Some("[package]"));
    assert!(doc.spans(0).is_empty());
}

#[test]
fn a_document_too_long_to_colour_keeps_its_text() {
    let line = "fn one() {}\n";
    let text = line.repeat(crate::MAX_HIGHLIGHT_BYTES / line.len() + 1);
    let doc = Document::new("big.rs", &text);
    assert_eq!(doc.language(), Some(Language::Rust));
    assert!(!doc.is_highlighted());
    assert_eq!(doc.line(0).as_deref(), Some("fn one() {}"));
}
