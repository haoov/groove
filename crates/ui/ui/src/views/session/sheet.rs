//! The review sheet: one write shown whole over the work half, with its two answers.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{ApprovalId, Ask};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Button;

/// The write under review, over the workspace and the sidebar.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(ask) = ui.examining().and_then(|id| found(app, id)) else {
        return;
    };
    ctx.layer();
    let area = area(ctx);
    groove_ui_kit::shape::ground(ctx, area, Ground::Work);
    ctx.hit(area, Target::Sheet);
    let line = ctx.tokens.line;
    let mut body = area.pad(Edges::all(ctx.tokens.md));
    let asks = crate::views::rail::asks_to(&ask.op);
    Label::new(&asks, ctx.styles.strong(Role::Attention)).draw(ctx, body.take_top(line));
    let place = place(app, &ask);
    Label::new(&place, ctx.styles.small(Role::Muted)).draw(ctx, body.take_top(line));
    body.take_top(ctx.tokens.sm);
    let foot = body.take_bottom(ctx.tokens.row);
    said(ctx, body, &ask.text);
    answers(ctx, foot, &ask.id);
}

/// The workspace and the sidebar together, from the header down.
fn area(ctx: &Ctx) -> Rect {
    let layout = &ctx.app.layout;
    let right = layout.workspace.right().max(layout.sidebar.right());
    let bottom = layout.workspace.bottom().max(layout.sidebar.bottom());
    let (x, y) = (layout.header.x, layout.header.y);
    Rect::new(x, y, right - x, bottom - y)
}

/// The write this id names, in whichever session waits on it.
fn found(app: &AppState, id: &ApprovalId) -> Option<Ask> {
    app.agent
        .agents
        .iter()
        .flat_map(|(_, agent)| agent.activity.asks.iter())
        .find(|ask| &ask.id == id)
        .cloned()
}

/// The repo and the branch the write acts in, when it acts in a worktree.
fn place(app: &AppState, ask: &Ask) -> String {
    let worktree = ask.worktree.as_ref().and_then(|id| {
        app.session
            .open
            .iter()
            .flat_map(|open| open.worktrees.iter())
            .find(|one| &one.id == id)
    });
    match worktree {
        Some(one) => format!("{} · {}", one.repo.as_str(), one.branch),
        None => ask.subject.clone(),
    }
}

/// The text of the write, a line a row, cut where the sheet ends.
fn said(ctx: &mut Ctx, mut rect: Rect, text: &str) {
    let (style, line) = (ctx.styles.code(Role::Text), ctx.tokens.line);
    for one in text.lines() {
        if rect.h < line {
            break;
        }
        Label::new(one, style).draw(ctx, rect.take_top(line));
    }
}

/// Approve in peach, and refuse, from the sheet's end.
fn answers(ctx: &mut Ctx, line: Rect, id: &ApprovalId) {
    let (mut room, raised) = (line, ctx.styles.raised());
    let refuse = Button::new("Refuse", Target::Refuse(id.clone()), Role::Muted, raised);
    refuse.right(ctx, &mut room, ctx.tokens.xs);
    let approve = Button::new(
        "Approve",
        Target::Approve(id.clone()),
        Role::Attention,
        raised,
    );
    approve.right(ctx, &mut room, ctx.tokens.xs);
}
