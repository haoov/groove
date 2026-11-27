//! The popup's lines: the diagnostics, then the hover's Markdown, then the footer.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Document, Hovered};
use groove_types::{Diagnostic, Severity};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::blocks;
use groove_ui_kit::markdown::parse::{Block, Kind};
use groove_ui_kit::widgets::{Piece, Said};

use crate::Ui;

/// The lines, and the role the card is ringed in: the worst diagnostic's, else muted.
pub(super) fn said(app: &AppState, ui: &Ui, hovered: &Hovered) -> (Vec<Said>, Role) {
    let mut out = Vec::new();
    let found = covering(app, hovered);
    let ring = found.first().map_or(Role::Muted, |one| role(one.severity));
    for (at, found) in found.into_iter().enumerate() {
        if at > 0 {
            out.push(Said::Gap);
        }
        out.extend(diagnosed(&found));
    }
    let Some(text) = hovered.said.as_deref().filter(|one| !one.trim().is_empty()) else {
        return (out, ring);
    };
    if !out.is_empty() {
        out.push(Said::Rule);
    }
    let word = if hovered.told { "definition" } else { "hover" };
    out.push(head(word, hovered.at.line, Role::Muted));
    if hovered.told {
        out.push(plain(text, Role::Faint));
        return (out, ring);
    }
    let read = ui
        .spoken
        .of((&hovered.path, text), || markdown(&hovered.path, text));
    out.extend(read.iter().cloned());
    out.push(Said::Rule);
    out.push(plain("ctrl+click: go to definition", Role::Ghost));
    (out, ring)
}

/// `WARNING · LINE 31`, the line counted from one.
fn head(word: &str, line: usize, role: Role) -> Said {
    Said::Head {
        word: word.to_string(),
        note: format!("line {}", line + 1),
        role,
    }
}

/// A diagnostic: its head, its first line as the title, the rest muted, then its source.
fn diagnosed(found: &Diagnostic) -> Vec<Said> {
    let mut lines = found.message.lines().filter(|line| !line.trim().is_empty());
    let title = lines.next().unwrap_or_default();
    let severity = found.severity;
    let mut out = vec![
        head(kind(severity), found.start.line, role(severity)),
        prose(&blocks(title), Role::Text),
    ];
    out.extend(lines.map(|line| prose(&blocks(line.trim()), Role::Muted)));
    let named: Vec<&str> = [found.source.as_deref(), found.code.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    if !named.is_empty() {
        out.push(Said::Gap);
        out.push(plain(&named.join(" · "), Role::Ghost));
    }
    out
}

/// The hover's blocks: prose as it reads, each run of code lines coloured as `path` is.
fn markdown(path: &str, text: &str) -> Vec<Said> {
    let mut out = Vec::new();
    let mut code: Vec<String> = Vec::new();
    for block in blocks(text) {
        let block = block.flattened();
        if block.kind == Kind::Code {
            code.extend(block.spans.iter().map(|span| span.text.clone()));
            continue;
        }
        out.extend(coloured(path, std::mem::take(&mut code)));
        out.push(match block.kind {
            Kind::Rule => Said::Rule,
            Kind::Heading(_) => prose(std::slice::from_ref(&block), Role::Text),
            _ => prose(std::slice::from_ref(&block), Role::Muted),
        });
    }
    out.extend(coloured(path, code));
    out
}

fn coloured(path: &str, lines: Vec<String>) -> Vec<Said> {
    let mut lines = lines;
    while lines.last().is_some_and(|one| one.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Vec::new();
    }
    let document = Document::new(path, &lines.join("\n"));
    let colours = document.colours(0..lines.len());
    lines
        .into_iter()
        .enumerate()
        .map(|(at, text)| Said::Code {
            spans: colours.of(at).to_vec(),
            text,
        })
        .collect()
}

/// The blocks' spans as one line, a list's items marked.
fn prose(blocks: &[Block], role: Role) -> Said {
    let mut pieces = Vec::new();
    for block in blocks {
        if let Kind::Item(_) = block.kind {
            pieces.push(Piece {
                text: "• ".into(),
                code: false,
            });
        }
        pieces.extend(block.spans.iter().map(|span| Piece {
            text: span.text.clone(),
            code: span.code,
        }));
    }
    Said::Prose {
        pieces,
        role,
        mark: None,
    }
}

fn plain(text: &str, role: Role) -> Said {
    let pieces = vec![Piece {
        text: text.to_string(),
        code: false,
    }];
    Said::Prose {
        pieces,
        role,
        mark: None,
    }
}

fn covering(app: &AppState, hovered: &Hovered) -> Vec<Diagnostic> {
    let Some(worktree) = app.workspace.worktree.as_ref() else {
        return Vec::new();
    };
    let mut found: Vec<Diagnostic> = app
        .workspace
        .servers
        .diagnostics_on(worktree, &hovered.path, hovered.at.line)
        .iter()
        .filter(|one| one.start <= hovered.at && hovered.at <= one.end)
        .cloned()
        .collect();
    found.sort_by_key(|one| one.severity);
    found
}

fn role(severity: Severity) -> Role {
    match severity {
        Severity::Error => Role::Bad,
        Severity::Warning => Role::Warn,
        Severity::Information | Severity::Hint => Role::Muted,
    }
}

fn kind(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Information => "info",
        Severity::Hint => "hint",
    }
}
