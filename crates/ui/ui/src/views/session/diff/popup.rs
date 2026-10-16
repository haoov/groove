//! What the language servers say of one spot of the open file, in a panel under it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Hovered;
use groove_gfx::Rect;
use groove_types::{Caret, Diagnostic, Severity};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::REST_MS;
use groove_ui_kit::widgets::{Said, popup};

use crate::ctx::Ctx;
use crate::{Focus, Ui};

/// A spot of an open file the pointer stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointed {
    pub path: String,
    pub at: Caret,
}

/// Under the caret's column of `line`, for a popup asked from the keyboard to stand at.
pub(super) fn anchor(ctx: &mut Ctx, line: Rect, column: usize) {
    let chars = ctx.app.hits.chars();
    let x = chars.left + column as f32 * chars.advance;
    ctx.app.hits.caret_at((x, line.bottom()));
}

/// The popup, while the spot it speaks of is still where the pointer or the caret rests.
pub(crate) fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(hovered) = app.workspace.servers.hovered.as_ref() else {
        return;
    };
    let Some(at) = place(ctx, app, ui, hovered) else {
        return;
    };
    let lines = said(app, hovered);
    if lines.is_empty() {
        return;
    }
    let parts: Vec<Said> = lines
        .iter()
        .map(|(text, role, code)| Said {
            text,
            role: *role,
            code: *code,
        })
        .collect();
    popup(ctx, at, &parts);
}

/// Under the caret while it stays on the spot, else under the pointer once it has rested there.
fn place(ctx: &Ctx, app: &AppState, ui: &Ui, hovered: &Hovered) -> Option<(f32, f32)> {
    if hovered.caret {
        let open = app.workspace.active()?;
        let here = open.path == hovered.path && open.new.caret() == hovered.at;
        return (here && ui.focus == Focus::Workspace).then(|| ctx.app.hits.caret())?;
    }
    let here = Pointed {
        path: hovered.path.clone(),
        at: hovered.at,
    };
    let below = (ui.at.0, ui.at.1 + ctx.tokens.line);
    if hovered.told {
        return (ui.session.pointed.as_ref() == Some(&here)).then_some(below);
    }
    let (still, since) = ui.session.pointer_rest.as_ref()?;
    let asked = still.path == hovered.path && still.at == hovered.at;
    let rested = ctx.tick >= since + REST_MS && ui.session.pointed.as_ref() == Some(still);
    (asked && rested).then_some(below)
}

/// The diagnostics on the spot, worst first, then what the hover said, its code blocks as code.
fn said(app: &AppState, hovered: &Hovered) -> Vec<(String, Role, bool)> {
    let mut lines = Vec::new();
    for found in covering(app, hovered) {
        lines.push((found.message.clone(), role(found.severity), false));
        let named: Vec<&str> = [found.source.as_deref(), found.code.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        if !named.is_empty() {
            lines.push((named.join(" · "), Role::Ghost, false));
        }
    }
    let mut code = false;
    let role = match hovered.told {
        true => Role::Faint,
        false => Role::Text,
    };
    for line in hovered.said.as_deref().unwrap_or_default().lines() {
        if line.trim_start().starts_with("```") {
            code = !code;
            continue;
        }
        lines.push((line.to_string(), role, code));
    }
    lines
}

fn covering(app: &AppState, hovered: &Hovered) -> Vec<Diagnostic> {
    let Some(worktree) = app.workspace.worktree.as_ref() else {
        return Vec::new();
    };
    let mut found: Vec<Diagnostic> = app
        .workspace
        .servers
        .diagnostics(worktree, &hovered.path)
        .into_iter()
        .filter(|one| one.start <= hovered.at && hovered.at <= one.end)
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
