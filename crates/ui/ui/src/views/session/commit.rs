//! The commit box under the changed files: the index, the action, the message.

use groove_controllers::{AppState, Command, delivery, workspace};
use groove_gfx::{Edges, Rect};
use groove_types::{FileDiff, Standing, Step};

use crate::components::{Gutters, Line, Rows, code};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Focus, Losing, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, counts};

/// The counts and what commits them on one line, the message under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    groove_ui_kit::shape::rule_above(ctx, rect, ctx.styles.line());
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
            across: 0.0,
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

/// What the worktree most wants doing next, from what it holds and what the forge said.
fn step(app: &AppState) -> Option<Step> {
    let files = super::files::changed(app);
    let status = status(app);
    let standing = Standing {
        changed: !files.is_empty(),
        staged: count(files, true),
        worded: !app.workspace.message.text().trim().is_empty(),
        ahead: status.ahead,
        behind: status.behind,
        wants_mr: wants_mr(app),
    };
    standing.next()
}

/// The command the box's button runs now.
pub(crate) fn primary(app: &AppState) -> Option<Command> {
    Some(match step(app)? {
        Step::Commit { .. } => Command::Workspace(workspace::Command::Commit),
        Step::Push => Command::Workspace(workspace::Command::Push),
        Step::Pull => Command::Workspace(workspace::Command::Pull),
        Step::OpenMr => Command::Delivery(delivery::Command::CreateMr),
    })
}

/// Whether the branch is landed and has no merge request of its own yet.
fn wants_mr(app: &AppState) -> bool {
    let Some(worktree) = app.session.selected_worktree() else {
        return false;
    };
    app.delivery.poll.knows(&worktree.id) && !app.delivery.has_mr(&worktree.id)
}

fn label(step: Step) -> &'static str {
    match step {
        Step::Push => "push",
        Step::Pull => "pull",
        Step::OpenMr => "open mr",
        Step::Commit { .. } => "commit",
    }
}

/// One button for what to do now, a caret for the rest; returns where it starts.
fn acts(ctx: &mut Ctx, app: &AppState, line: Rect) -> f32 {
    let act = step(app);
    let can = act.is_some_and(|one| one != Step::Commit { ready: false });
    let role = match can {
        true => Role::Text,
        false => Role::Faint,
    };
    let word = act.map_or("commit", label);
    let (action, raised) = (ctx.styles.action(), ctx.styles.raised());
    let rest = Button::icon(Mark::Down, Mark::UPWARDS, Target::Actions, Role::Muted);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    rest.ground(raised).hover(action).right(ctx, &mut room, 0.0);
    let run = Button::new(word, Target::Do, role, raised)
        .lit(role)
        .hover(action);
    run.inert(!can).right(ctx, &mut room, 0.0).x
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
