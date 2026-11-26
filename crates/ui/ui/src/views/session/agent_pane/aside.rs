//! The agent's sidebar in the modes layout: its model, what it uses, and the agents it sent.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::from_top;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::glass_hoverable;

use super::{model, subagent};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::cap;

/// The sidebar's body from the top of `room`: the model, the gauges, the agents sent.
pub fn body(ctx: &mut Ctx, app: &AppState, room: &mut Rect) {
    let Some(session) = app.session.selected.as_ref() else {
        return;
    };
    let Some(agent) = app.agent.agent(session) else {
        return;
    };
    let chat = app.agent.chat(session);
    let gauges = model::gauges(chat, agent, ctx.now);
    let line = from_top(room, ctx.tokens.row, ctx.tokens.xs);
    glass_hoverable(ctx, line, Target::ModelButton);
    let mut line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    cap(ctx, &mut line, "model", Role::Ghost);
    let named = model::named(app).unwrap_or_else(|| "—".into());
    Label::new(&named, ctx.styles.body(Role::Text)).draw(ctx, line);
    let side = Edges::across(ctx.tokens.md, ctx.tokens.md);
    if !gauges.is_empty() {
        heading(ctx, room, "usage");
    }
    for one in &gauges {
        model::gauge(
            ctx,
            from_top(room, ctx.tokens.row * 2.0, 0.0).pad(side),
            one,
        );
    }
    let sent = subagent::docked(chat);
    if !sent.is_empty() {
        heading(ctx, room, "agents");
    }
    for one in sent {
        let mut line = from_top(room, ctx.tokens.row, 0.0).pad(side);
        glass_hoverable(ctx, line, Target::Subagent(one.id.clone()));
        let (word, role) = subagent::state(one.state);
        groove_ui_kit::widgets::cap_right(ctx, &mut line, word, role);
        let said = format!("{} · {}", one.kind, one.task);
        Label::new(&said, ctx.styles.body(Role::Text)).draw(ctx, line);
    }
}

fn heading(ctx: &mut Ctx, room: &mut Rect, word: &str) {
    let line = from_top(room, ctx.tokens.row, ctx.tokens.xs);
    let mut line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    cap(ctx, &mut line, word, Role::Ghost);
}
