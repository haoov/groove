//! The commit box under the changed files: the index, the action, the message.

use groove_controllers::{AppState, Command, delivery, workspace};
use groove_gfx::{Edges, Rect};
use groove_types::FileDiff;

use crate::components::{Gutters, Line, Rows, code};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Focus, Losing, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, counts, mark_button};

/// The counts and what commits them on one line, the message under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - ctx.tokens.hairline, rect.w, 0.0),
        ctx.styles.line(),
    );
    if app.workspace.commit.is_some() {
        return showing(ctx, rect, app);
    }
    let mut message = rect;
    let top = message.take_top(ctx.tokens.row);
    if ui.losing() == Some(&Losing::Everything) {
        super::files::asking(ctx, top, "discard every change?");
    } else {
        let acts = acts(ctx, app, top);
        state_of(ctx, app, top.until(acts));
    }
    typed(ctx, app, ui, message);
}

/// The commit the surface shows, and what puts the working tree back.
pub(super) fn showing(ctx: &mut Ctx, rect: Rect, app: &AppState) {
    let Some(one) = app.workspace.commit.as_ref() else {
        return;
    };
    let line = Rect {
        h: ctx.tokens.row,
        ..rect
    };
    ctx.quad(line, ctx.styles.band());
    let (raised, action, sm) = (ctx.styles.raised(), ctx.styles.action(), ctx.tokens.sm);
    let working = Button::new("working tree", Target::Working, Role::Muted, raised).hover(action);
    let mut room = line.pad(Edges::across(ctx.tokens.md, sm));
    working.right(ctx, &mut room, sm);
    Label::new(&one.short_sha, ctx.styles.code(Role::Muted)).left(ctx, &mut room, sm);
    let said = one.message.lines().next().unwrap_or_default();
    Label::new(said, ctx.styles.body(Role::Text)).draw(ctx, room);
}

/// What the branch is ahead by, what the index holds, and what is still waiting.
fn state_of(ctx: &mut Ctx, app: &AppState, line: Rect) {
    let files = super::files::changed(app);
    let items = [
        (Mark::Ahead, status(app).ahead, Role::Working),
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
    let held = ui.session.composing && !ui.session.typing();
    let composing = held && ui.focus == Focus::Sidebar;
    ctx.quad(rect, ctx.styles.ground());
    ctx.border(rect, ctx.styles.border());
    let box_ = rect.pad(Edges::all(ctx.tokens.xs));
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
        let line = Rect {
            h: ctx.tokens.line,
            ..box_
        };
        Label::new("a message", ctx.styles.code(Role::Ghost)).draw(ctx, line);
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
fn acts(ctx: &mut Ctx, app: &AppState, line: Rect) -> f32 {
    let act = primary(app);
    let can = act.as_ref().is_some_and(|act| ready(app, act));
    let role = match can {
        true => Role::Text,
        false => Role::Faint,
    };
    let word = act.as_ref().map(label).unwrap_or("commit");
    let (action, raised) = (ctx.styles.action(), ctx.styles.raised());
    let caret = (Mark::Down, Mark::UPWARDS, Role::Muted);
    let arrow = mark_button(ctx, line, Target::Actions, caret, (raised, action));
    let run = Button::new(word, Target::Do, role, raised)
        .lit(role)
        .hover(action);
    let mut left = line.until(arrow.x);
    run.inert(!can).right(ctx, &mut left, 0.0).x
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
