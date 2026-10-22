//! The agent's row under its screen: the skills menu, in glass.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::band;
use groove_ui_kit::widgets::{Button, glass_pill};

/// What the agent can be sent.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    offered(ctx, ctx.app.layout.agent_bar, open);
}

/// The skills menu at the row's start, a gap above the foot.
fn offered(ctx: &mut Ctx, line: Rect, open: &Open) {
    let (sm, xs, tall) = (ctx.tokens.sm, ctx.tokens.xs, ctx.tokens.row);
    let foot = band(line, line.h - sm - tall, tall);
    let mut room = foot.pad(Edges::across(sm + xs, sm + xs));
    let target = Target::Skills(open.session.id.clone());
    let skills = Button::new("skills", target, Role::Muted, ctx.styles.ground());
    let skills = skills.flat().caret(Mark::UPWARDS).lit(Role::Text);
    let word = skills.left(ctx, &mut room, 0.0);
    glass_pill(ctx, word, xs);
}
