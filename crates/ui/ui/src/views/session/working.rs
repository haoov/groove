//! The agent's bar at the middle's foot while it works, on every mode: what runs now, how long.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::AgentStatus;
use groove_types::chat::{Chat, Entry, Glance};
use groove_ui_kit::shape::{Panel, from_bottom};
use groove_ui_kit::widgets::glass_hoverable;

use super::Mode;
use super::agent_pane::steps::running;
use crate::ctx::Ctx;
use crate::hit::Target;

/// Whether the session's agent works now, and was not just interrupted.
pub(crate) fn works(app: &AppState, open: &Open) -> bool {
    let agent = app.agent.agent(&open.session.id);
    let last = app
        .agent
        .chat(&open.session.id)
        .and_then(|one| one.entries().last());
    let stopped = matches!(last, Some(Entry::Interrupted));
    !stopped && agent.is_some_and(|one| one.activity.status == AgentStatus::Working)
}

/// The room the bar takes above the middle's foot, its gap over it included.
pub(crate) fn tall(ctx: &Ctx) -> f32 {
    ctx.tokens.row + ctx.tokens.sm * 2.0
}

/// The bar, as the conversation's running row; a click brings `AGENT` up.
pub fn draw(ctx: &mut Ctx, app: &AppState, open: &Open) {
    if !works(app, open) {
        return;
    }
    let chat = app.agent.chat(&open.session.id);
    let glance = match chat.and_then(|one| one.entries().last()) {
        Some(Entry::Glance(glance)) => glance.clone(),
        _ => Glance {
            since: chat.and_then(Chat::now),
            ..Glance::default()
        },
    };
    let bar = place(ctx);
    ctx.layer();
    Panel::default().ground(ctx.styles.ground()).draw(ctx, bar);
    running(ctx, bar, &glance);
    glass_hoverable(ctx, bar, Target::ModeTab(Mode::Agent));
}

fn place(ctx: &Ctx) -> Rect {
    let mut middle = ctx.app.layout.agent.pad(Edges {
        bottom: ctx.tokens.sm,
        ..Edges::across(ctx.tokens.xl, ctx.tokens.xl)
    });
    from_bottom(&mut middle, ctx.tokens.row, 0.0)
}
