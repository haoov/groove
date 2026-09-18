//! The commit box under the changed files: what the index holds, what commits it,
//! and the message.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::FileDiff;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Gutters, Line, Rows, button, code, counts, hairline, row};
use crate::{Focus, Ui};

/// The counts and what commits them on one line, the message under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - ctx.tokens.hairline, rect.w, 0.0),
        ctx.styles.line(),
    );
    let top = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.row);
    let commits = commit(ctx, app, ui, top);
    state_of(ctx, app, Rect::new(top.x, top.y, commits - top.x, top.h));
    let message = Rect::new(rect.x, top.bottom(), rect.w, rect.bottom() - top.bottom());
    typed(ctx, app, ui, message);
}

/// What the index holds against what is still waiting.
fn state_of(ctx: &mut Ctx, app: &AppState, line: Rect) {
    let files = super::files::changed(app);
    let items = [
        (Mark::Staged, count(files, true), Role::Ok),
        (Mark::Modified, count(files, false), Role::Warn),
    ];
    counts(ctx, line, line.x + ctx.tokens.md, &items);
}

fn count(files: &[FileDiff], staged: bool) -> u32 {
    files
        .iter()
        .filter(|file| file.staged == Some(staged))
        .count() as u32
}

/// The message, on the same surface a file is edited on.
fn typed(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    let composing = ui.session.composing && ui.focus == Focus::Sidebar;
    ctx.quad(rect, ctx.styles.inner());
    let buffer = &app.workspace.message;
    let caret = buffer.caret();
    let held: Vec<String> = (0..buffer.lines().max(1))
        .map(|at| buffer.line(at).unwrap_or_default().to_string())
        .collect();
    let lines: Vec<Line<'_>> = held
        .iter()
        .enumerate()
        .map(|(at, text)| {
            let on = composing && caret.line == at;
            Line::new(text).caret(on.then_some(caret.column))
        })
        .collect();
    ctx.clipped(rect, |ctx| {
        let rows = Rows {
            lines: &lines,
            first: 0,
            gutters: Gutters {
                cells: 0,
                digits: 0,
            },
        };
        code(ctx, rect, rows, 0.0);
    });
    ctx.hit(rect, Target::Message);
    if buffer.text().is_empty() && !composing {
        let style = ctx.styles.code(Role::Ghost);
        row(ctx, rect, ctx.tokens.md, "a message", style);
    }
}

/// What commits. Returns where it starts, so the counts know their room.
fn commit(ctx: &mut Ctx, app: &AppState, ui: &Ui, line: Rect) -> f32 {
    let ready = !app.workspace.message.text().trim().is_empty();
    let role = match ready {
        true => Role::Text,
        false => Role::Faint,
    };
    let style = ctx.styles.body(role);
    let on_it = ui.hover.as_ref() == Some(&Target::Commit);
    let ground = match on_it && ready {
        true => ctx.styles.action(),
        false => ctx.styles.raised(),
    };
    let box_ = button(ctx, line, "commit", style, Some(ground));
    if ready {
        ctx.hit(box_, Target::Commit);
    }
    box_.x
}
