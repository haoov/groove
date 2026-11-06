//! The popup's lines: the diagnostics, then the hover's Markdown, then the footer.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Document, Hovered};
use groove_types::{Diagnostic, Severity};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::blocks;
use groove_ui_kit::markdown::parse::{Block, Kind};
use groove_ui_kit::widgets::{Piece, Said};

use crate::Ui;

pub(super) fn said(app: &AppState, ui: &Ui, hovered: &Hovered) -> Vec<Said> {
    let mut out = Vec::new();
    for (at, found) in covering(app, hovered).into_iter().enumerate() {
        if at > 0 {
            out.push(Said::Gap);
        }
        out.extend(diagnosed(&found));
    }
    let Some(text) = hovered.said.as_deref().filter(|one| !one.trim().is_empty()) else {
        return out;
    };
    if hovered.told {
        out.push(plain(text, Role::Faint));
        return out;
    }
    out.push(Said::Rule);
    let read = ui
        .spoken
        .of((&hovered.path, text), || markdown(&hovered.path, text));
    out.extend(read.iter().cloned());
    out.push(Said::Rule);
    out.push(plain("ctrl+click: go to definition", Role::Ghost));
    out
}

/// A diagnostic: its first line as a coloured title, the rest as plain text, then its source.
fn diagnosed(found: &Diagnostic) -> Vec<Said> {
    let mut lines = found.message.lines().filter(|line| !line.trim().is_empty());
    let title = lines.next().unwrap_or_default();
    let mut out = vec![prose(
        &blocks(title),
        role(found.severity),
        mark(found.severity),
    )];
    out.extend(lines.map(|line| prose(&blocks(line.trim()), Role::Muted, None)));
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
            Kind::Heading(_) => prose(std::slice::from_ref(&block), Role::Text, None),
            _ => prose(std::slice::from_ref(&block), Role::Muted, None),
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
fn prose(blocks: &[Block], role: Role, mark: Option<Mark>) -> Said {
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
    Said::Prose { pieces, role, mark }
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
        Severity::Information => Role::Working,
        Severity::Hint => Role::Faint,
    }
}

fn mark(severity: Severity) -> Option<Mark> {
    match severity {
        Severity::Error => Some(Mark::Errors),
        Severity::Warning => Some(Mark::Warnings),
        _ => None,
    }
}
