//! The review sheet: one write shown whole over the work half or the middle, with its two answers.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{ApprovalId, Ask};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, band, from_bottom, from_top, top_rule};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::glass_pane;

/// The selected session's write under review, as the agent's own asks stand: glass ringed in
/// the ask's colour, a tinted head naming what it asks and where, the write's text, the answers.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(ask) = ui.examining().and_then(|id| found(app, id)) else {
        return;
    };
    ctx.layer();
    let under = area(ctx, &ask.text);
    ctx.hit(under, Target::Sheet);
    glass_pane(ctx, under);
    Panel::default()
        .border(ctx.styles.band_edge(Role::Attention))
        .draw(ctx, under);
    let mut room = under;
    let top = from_top(&mut room, ctx.tokens.bar, 0.0);
    Panel::default()
        .ground(ctx.styles.band_tint(Role::Attention))
        .draw(ctx, top);
    let mut top = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let asks = crate::views::rail::asks_to(&ask.op);
    groove_ui_kit::widgets::cap(ctx, &mut top, &asks, Role::Attention);
    let titled = app.session.selected().map(|one| one.session.title.as_str());
    let place = match titled {
        Some(title) => format!("{title} · {}", place(app, &ask)),
        None => place(app, &ask),
    };
    Label::new(&place, ctx.styles.log(Role::Ghost))
        .cut_start()
        .draw(ctx, top);
    let foot = from_bottom(&mut room, ctx.tokens.bar, 0.0);
    top_rule(ctx, foot, ctx.styles.panel_edge());
    answers(
        ctx,
        foot.pad(Edges::across(ctx.tokens.md, ctx.tokens.md)),
        &ask.id,
    );
    said(ctx, room.pad(Edges::all(ctx.tokens.md)), &ask.text);
}

/// Over the middle's top, as tall as the text with its head and its answers, never past half
/// the middle.
fn area(ctx: &Ctx, text: &str) -> Rect {
    let (layout, tokens) = (&ctx.app.layout, &ctx.tokens);
    let middle = layout.agent.pad(Edges::xy(tokens.xl, tokens.lg));
    let lines = text.lines().count().max(1) as f32;
    let tall = lines * tokens.code_line + tokens.md * 2.0 + tokens.bar * 2.0;
    band(middle, 0.0, tall.min(middle.h / 2.0))
}

/// The write this id names, when the selected session waits on it.
pub(crate) fn found(app: &AppState, id: &ApprovalId) -> Option<Ask> {
    let session = app.session.selected.as_ref()?;
    let asks = &app.agent.activity(session)?.asks;
    asks.iter().find(|ask| &ask.id == id).cloned()
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

/// The text of the write in mono, its first line whole, the rest faint, cut where the sheet ends.
fn said(ctx: &mut Ctx, rect: Rect, text: &str) {
    let line = ctx.tokens.code_line;
    ctx.clipped(rect, |ctx| {
        for (at, one) in text.lines().enumerate() {
            let role = if at == 0 { Role::Text } else { Role::Faint };
            let row = band(rect, at as f32 * line, line);
            Label::new(one, ctx.styles.code(role)).draw(ctx, row);
        }
    });
}

/// Approve on Enter in the ask's colour, Refuse on Escape, from the sheet's end.
fn answers(ctx: &mut Ctx, line: Rect, id: &ApprovalId) {
    let mut room = line;
    let answer = groove_ui_kit::widgets::answer;
    answer(
        ctx,
        &mut room,
        ("Approve", "ENTER"),
        (Target::Approve(id.clone()), true),
    );
    answer(
        ctx,
        &mut room,
        ("Refuse", "ESC"),
        (Target::Refuse(id.clone()), false),
    );
}
