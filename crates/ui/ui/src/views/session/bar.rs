//! The agent's row under its screen: the terminal in place of the conversation, in glass.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::band;
use groove_ui_kit::widgets::{Button, glass_pill};

/// Which of the agent's two faces shows.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui) {
    if app.session.selected().is_none() {
        return;
    }
    viewed(ctx, ctx.app.layout.agent_bar, ui.agent.terminal);
}

/// The other face at the row's start: the terminal, or the conversation back.
fn viewed(ctx: &mut Ctx, line: Rect, terminal: bool) {
    let (sm, xs, tall) = (ctx.tokens.sm, ctx.tokens.xs, ctx.tokens.bar);
    let foot = band(line, line.h - sm - tall, tall);
    let mut room = foot.pad(Edges::across(sm + xs, sm + xs));
    let word = toggle(ctx, terminal).left(ctx, &mut room, 0.0);
    glass_pill(ctx, word, xs);
}

fn toggle(ctx: &Ctx, terminal: bool) -> Button<'static, Target> {
    let word = if terminal { "conversation" } else { "terminal" };
    let button = Button::new(word, Target::AgentView, Role::Muted, ctx.styles.ground());
    button.flat().lit(Role::Text)
}
