//! The commit box under the changed files: what the index holds, what commits it,
//! and the message.

use groove_controllers::{AppState, workspace};
use groove_gfx::Rect;
use groove_types::FileDiff;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Gutters, Line, Rows, box_in, button, code, counts, hairline, row, slot};
use crate::{Focus, Losing, Ui};

/// The counts and what commits them on one line, the message under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - ctx.tokens.hairline, rect.w, 0.0),
        ctx.styles.line(),
    );
    let top = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.row);
    if ui.discarding == Some(Losing::Everything) {
        super::files::asking(ctx, top, "discard every change?", ui);
    } else {
        let acts = acts(ctx, app, ui, top);
        state_of(ctx, app, ui, Rect::new(top.x, top.y, acts - top.x, top.h));
    }
    let message = Rect::new(rect.x, top.bottom(), rect.w, rect.bottom() - top.bottom());
    typed(ctx, app, ui, message);
}

/// What the branch is ahead by, what the index holds, and what is still waiting.
fn state_of(ctx: &mut Ctx, app: &AppState, ui: &Ui, line: Rect) {
    let files = super::files::changed(app);
    let ahead = ahead(app, ui);
    let items = [
        (Mark::Ahead, ahead, Role::Working),
        (Mark::Staged, count(files, true), Role::Ok),
        (Mark::Modified, count(files, false), Role::Warn),
    ];
    counts(ctx, line, line.x + ctx.tokens.md, &items);
}

/// Commits the branch has that its own head on origin does not.
fn ahead(app: &AppState, _ui: &Ui) -> u32 {
    status(app).ahead
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

/// What the worktree most wants doing, if anything.
pub(crate) fn primary(app: &AppState) -> Option<workspace::Command> {
    let files = super::files::changed(app);
    let ready = !app.workspace.message.text().trim().is_empty();
    let staged = count(files, true);
    let status = status(app);
    match (files.is_empty(), status.ahead, status.behind) {
        (false, _, _) if staged > 0 && ready => Some(workspace::Command::Commit),
        (false, _, _) => Some(workspace::Command::Commit),
        (true, ahead, _) if ahead > 0 => Some(workspace::Command::Push),
        (true, _, behind) if behind > 0 => Some(workspace::Command::Pull),
        _ => None,
    }
}

/// Whether the action can be taken now, or is only what the box would do next.
fn ready(app: &AppState, act: &workspace::Command) -> bool {
    match act {
        workspace::Command::Commit => {
            let staged = count(super::files::changed(app), true);
            staged > 0 && !app.workspace.message.text().trim().is_empty()
        }
        _ => true,
    }
}

fn label(act: &workspace::Command) -> &'static str {
    match act {
        workspace::Command::Push => "push",
        workspace::Command::Pull => "pull",
        _ => "commit",
    }
}

/// One button: what to do now, and a caret to everything else it could do.
/// Returns where it starts, so the counts know their room.
fn acts(ctx: &mut Ctx, app: &AppState, ui: &Ui, line: Rect) -> f32 {
    let act = primary(app);
    let can = act.as_ref().is_some_and(|act| ready(app, act));
    let role = match can {
        true => Role::Text,
        false => Role::Faint,
    };
    let style = ctx.styles.small(role);
    let word = act.as_ref().map(label).unwrap_or("commit");
    let caret = ctx.tokens.small;
    let (action, raised) = (ctx.styles.action(), ctx.styles.raised());
    let lit = |target: Target| match ui.hover.as_ref() == Some(&target) {
        true => action,
        false => raised,
    };
    let arrow = slot(ctx, line, caret, Some(lit(Target::Actions)));
    let middle = arrow.x + (arrow.w - caret) / 2.0;
    let box_ = box_in(arrow, middle, caret);
    let color = ctx.styles.color(Role::Muted);
    ctx.icon(box_, Mark::Down, Mark::UPWARDS, color);
    ctx.hit(arrow, Target::Actions);

    let left = Rect::new(line.x, line.y, arrow.x - line.x + ctx.tokens.sm, line.h);
    let word = button(ctx, left, word, style, Some(lit(Target::Do)));
    if can {
        ctx.hit(word, Target::Do);
    }
    let split = Rect::new(word.right(), word.y, ctx.tokens.hairline, word.h);
    ctx.quad(split, ctx.styles.border());
    word.x
}

/// What git says about the selected worktree.
fn status(app: &AppState) -> groove_types::WorktreeStatus {
    app.session
        .selected()
        .and_then(|open| {
            let worktree = open.selected_worktree()?;
            open.delivery_of(&worktree.id)
        })
        .map(|delivery| delivery.status)
        .unwrap_or_default()
}
