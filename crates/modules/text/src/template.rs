//! A template's second layer: YAML parsed over the text between its actions.

use ropey::Rope;
use tree_sitter::{Parser, Range, Tree};

use crate::language::Language;

/// The YAML the template's text nodes make, parsed as one document, from `old` when given.
pub(crate) fn yaml(text: &Rope, template: &Tree, old: Option<&Tree>) -> Option<Tree> {
    let ranges = texts(template);
    if ranges.is_empty() {
        return None;
    }
    let (grammar, _) = Language::Yaml.syntax()?;
    let mut parser = Parser::new();
    parser.set_language(grammar).ok()?;
    parser.set_included_ranges(&ranges).ok()?;
    crate::highlight::parse(&mut parser, text, old)
}

/// Every `text` node of the template, in order.
fn texts(template: &Tree) -> Vec<Range> {
    let mut out = Vec::new();
    let mut cursor = template.walk();
    loop {
        let node = cursor.node();
        if node.kind() == "text" {
            out.push(node.range());
        } else if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return out;
            }
        }
    }
}
