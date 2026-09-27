//! The commit box under the changed files: the index, the action, the message.

use groove_controllers::{AppState, Command, delivery, workspace};
use groove_gfx::Rect;
use groove_types::FileDiff;

use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{box_in, hairline};
use crate::text::row;
use crate::widgets::{Gutters, Line, Rows, button, code, counts, slot};
use crate::{Focus, Losing, Ui};

/// The counts and what commits them on one line, the message under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - ctx.tokens.hairline, rect.w, 0.0),
        ctx.styles.line(),
    );
    let top = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.row);
    if app.workspace.commit.is_some() {
        return showing(ctx, rect, app, ui);
    }
    if ui.losing() == Some(&Losing::Everything) {
        super::files::asking(ctx, top, "discard every change?", ui);
    } else {
        let acts = acts(ctx, app, ui, top);
        state_of(ctx, app, ui, Rect::new(top.x, top.y, acts - top.x, top.h));
    }
    let message = Rect::new(rect.x, top.bottom(), rect.w, rect.bottom() - top.bottom());
    typed(ctx, app, ui, message);
}

/// The commit the surface shows, and what puts the working tree back.
pub(super) fn showing(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui) {
    let Some(one) = app.workspace.commit.as_ref() else {
        return;
    };
    let line = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.row);
    ctx.quad(line, ctx.styles.band());
    let target = Target::Working;
    let on_it = ui.hover.as_ref() == Some(&target);
    let style = ctx.styles.small(Role::Muted);
    let ground = match on_it {
        true => ctx.styles.action(),
        false => ctx.styles.raised(),
    };
    let box_ = button(ctx, line, "working tree", style, Some(ground));
    ctx.hit(box_, target);
    let name = ctx.styles.code(Role::Muted);
    let width = ctx.measure(&one.short_sha, &name);
    row(ctx, line, ctx.tokens.md, &one.short_sha, name);
    let words = ctx.styles.body(Role::Text);
    let start = ctx.tokens.md + width + ctx.tokens.sm;
    let room = (box_.x - line.x - start - ctx.tokens.sm).max(0.0);
    let said = one.message.lines().next().unwrap_or_default();
    let text = crate::text::elide(ctx, said, &words, room);
    row(ctx, line, start, &text, words);
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
    let held = ui.session.composing && !ui.session.typing();
    let composing = held && ui.focus == Focus::Sidebar;
    ctx.quad(rect, ctx.styles.ground());
    ctx.border(rect, ctx.styles.border());
    let pad = ctx.tokens.xs;
    let box_ = Rect::new(
        rect.x + pad,
        rect.y + pad,
        rect.w - pad * 2.0,
        rect.h - pad * 2.0,
    );
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
    ctx.clipped(box_, |ctx| {
        let rows = Rows {
            lines: &lines,
            first: 0,
            gutters: Gutters {
                cells: 0,
                digits: 0,
            },
        };
        code(ctx, box_, rows, 0.0);
    });
    ctx.hit(box_, Target::Message);
    if buffer.text().is_empty() && !composing {
        let style = ctx.styles.code(Role::Ghost);
        let line = Rect::new(box_.x, box_.y, box_.w, ctx.tokens.line);
        row(ctx, line, 0.0, "a message", style);
    }
}

/// What the worktree most wants doing: commit, then push, then open an MR.
pub(crate) fn primary(app: &AppState) -> Option<Command> {
    let files = super::files::changed(app);
    let ready = !app.workspace.message.text().trim().is_empty();
    let staged = count(files, true);
    let status = status(app);
    match (files.is_empty(), status.ahead, status.behind) {
        (false, _, _) if staged > 0 && ready => {
            Some(Command::Workspace(workspace::Command::Commit))
        }
        (false, _, _) => Some(Command::Workspace(workspace::Command::Commit)),
        (true, ahead, _) if ahead > 0 => Some(Command::Workspace(workspace::Command::Push)),
        (true, _, behind) if behind > 0 => Some(Command::Workspace(workspace::Command::Pull)),
        (true, _, _) if wants_mr(app) => Some(Command::Delivery(delivery::Command::CreateMr)),
        _ => None,
    }
}

/// Whether the branch is landed and has no merge request of its own yet.
fn wants_mr(app: &AppState) -> bool {
    let Some(worktree) = app.session.selected_worktree() else {
        return false;
    };
    app.delivery.poll.knows(&worktree.id) && !app.delivery.has_mr(&worktree.id)
}

/// Whether the action can be taken now, or is only what the box would do next.
fn ready(app: &AppState, act: &Command) -> bool {
    match act {
        Command::Workspace(workspace::Command::Commit) => {
            let staged = count(super::files::changed(app), true);
            staged > 0 && !app.workspace.message.text().trim().is_empty()
        }
        _ => true,
    }
}

fn label(act: &Command) -> &'static str {
    match act {
        Command::Workspace(workspace::Command::Push) => "push",
        Command::Workspace(workspace::Command::Pull) => "pull",
        Command::Delivery(delivery::Command::CreateMr) => "open mr",
        _ => "commit",
    }
}

/// One button for what to do now, a caret for the rest; returns where it starts.
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
    word.x
}

/// What git says about the selected worktree.
fn status(app: &AppState) -> groove_types::WorktreeStatus {
    let Some(open) = app.session.selected() else {
        return Default::default();
    };
    open.selected_worktree()
        .map(|worktree| open.status_of(&worktree.id))
        .unwrap_or_default()
}
