//! The review sheet: one write shown whole over the work half, with its two answers.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{ApprovalId, Ask};

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::style::Role;
use crate::text::{elide, row};
use crate::widgets::Word;

/// The write under review, over the workspace and the sidebar.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(ask) = ui.examining().and_then(|id| found(app, id)) else {
        return;
    };
    ctx.layer();
    let area = area(ctx);
    ctx.quad(area, ctx.styles.ground());
    ctx.hit(area, Target::Sheet);
    let pad = ctx.tokens.md;
    let line = ctx.tokens.line;
    let mut at = Rect::new(area.x, area.y + pad, area.w, line);
    let verb = groove_controllers::agent_service::tools::verb(&ask.op);
    row(
        ctx,
        at,
        pad,
        &format!("asks to {verb}"),
        ctx.styles.strong(Role::Attention),
    );
    at.y += line;
    row(
        ctx,
        at,
        pad,
        &place(app, &ask),
        ctx.styles.small(Role::Muted),
    );
    at.y += line + ctx.tokens.sm;
    let foot = area.bottom() - pad - ctx.tokens.row;
    said(ctx, Rect::new(area.x, at.y, area.w, foot - at.y), &ask.text);
    answers(
        ctx,
        Rect::new(area.x, foot, area.w - pad, ctx.tokens.row),
        &ask.id,
    );
}

/// The workspace and the sidebar together, from the header down.
fn area(ctx: &Ctx) -> Rect {
    let layout = &ctx.layout;
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
fn said(ctx: &mut Ctx, rect: Rect, text: &str) {
    let style = ctx.styles.code(Role::Text);
    let pad = ctx.tokens.md;
    let line = ctx.tokens.line;
    let room = (rect.w - pad * 2.0).max(0.0);
    let fits = (rect.h / line).floor().max(0.0) as usize;
    for (at, one) in text.lines().take(fits).enumerate() {
        let shown = elide(ctx, one, &style, room);
        let at = Rect::new(rect.x, rect.y + at as f32 * line, rect.w, line);
        row(ctx, at, pad, &shown, style);
    }
}

/// Approve in peach, and refuse, from the sheet's end.
fn answers(ctx: &mut Ctx, line: Rect, id: &ApprovalId) {
    let (mut room, raised) = (line, ctx.styles.raised());
    let refuse = Word::new("Refuse", Target::Refuse(id.clone()), Role::Muted, raised);
    refuse.right(ctx, &mut room, ctx.tokens.xs);
    let approve = Word::new(
        "Approve",
        Target::Approve(id.clone()),
        Role::Attention,
        raised,
    );
    approve.right(ctx, &mut room, ctx.tokens.xs);
}
