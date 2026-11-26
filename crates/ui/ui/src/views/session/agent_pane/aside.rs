//! The agent's sidebar in the modes layout: its model, what it uses, and the agents it sent.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::from_top;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Heading, flat_row, glass_hoverable, panel_head, solid_pane};

use super::{model, subagent};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::cap;

/// The sidebar in `rect`: a head that folds it, the model, the usage gauges, then each agent sent.
pub fn draw(ctx: &mut Ctx, app: &AppState, rect: Rect, open: bool) {
    let Some(session) = app.session.selected.as_ref() else {
        return;
    };
    let Some(agent) = app.agent.agent(session) else {
        return;
    };
    solid_pane(ctx, rect);
    let mut room = rect;
    let top = from_top(&mut room, ctx.tokens.bar, 0.0);
    panel_head(ctx, top, !open);
    let row = flat_row(ctx, top, Target::AsideFold, false);
    Heading::new("agent").fold(open).draw(ctx, row);
    if !open {
        return;
    }
    let chat = app.agent.chat(session);
    let gauges = model::gauges(chat, agent, ctx.now);
    let line = from_top(&mut room, ctx.tokens.row, ctx.tokens.xs);
    glass_hoverable(ctx, line, Target::ModelButton);
    let mut line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    cap(ctx, &mut line, "model", Role::Ghost);
    let named = model::named(app).unwrap_or_else(|| "—".into());
    Label::new(&named, ctx.styles.body(Role::Text)).draw(ctx, line);
    let mut room = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    if !gauges.is_empty() {
        heading(ctx, &mut room, "usage");
    }
    for one in &gauges {
        model::gauge(ctx, from_top(&mut room, ctx.tokens.row * 2.0, 0.0), one);
    }
    let sent = subagent::docked(chat);
    if !sent.is_empty() {
        heading(ctx, &mut room, "agents");
    }
    for one in sent {
        let mut line = from_top(&mut room, ctx.tokens.row, 0.0);
        glass_hoverable(ctx, line, Target::Subagent(one.id.clone()));
        let (word, role) = subagent::state(one.state);
        groove_ui_kit::widgets::cap_right(ctx, &mut line, word, role);
        let said = format!("{} · {}", one.kind, one.task);
        Label::new(&said, ctx.styles.body(Role::Text)).draw(ctx, line);
    }
}

fn heading(ctx: &mut Ctx, room: &mut Rect, word: &str) {
    let mut line = from_top(room, ctx.tokens.row, ctx.tokens.xs);
    cap(ctx, &mut line, word, Role::Ghost);
}
