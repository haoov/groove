use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::AgentStatus;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{row, screen};

/// The agent's terminal, or why there is none.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let pane = ctx.layout.agent;
    let (ground, line, hairline) = (ctx.styles.ground(), ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(pane, ground);
    ctx.hit(pane, Target::Agent);
    ctx.quad(
        Rect::new(pane.right() - hairline, pane.y, hairline, pane.h),
        line,
    );

    let Some(agent) = app.agent.agent(&open.session.id) else {
        return note(ctx, "starting the agent…");
    };
    match (&agent.terminal, &agent.activity.status) {
        (Some(terminal), _) => {
            let origin = ctx.layout.agent_origin(&ctx.tokens);
            let grid = terminal.screen();
            screen(ctx, pane, origin, &grid);
        }
        (None, AgentStatus::Error { message }) => note(ctx, message),
        (None, _) => note(ctx, "starting the agent…"),
    }
}

fn note(ctx: &mut Ctx, text: &str) {
    let style = ctx.styles.body(Role::Faint);
    let (pad, pane) = (ctx.tokens.md, ctx.layout.agent);
    let rect = Rect::new(pane.x, pane.y + ctx.tokens.sm, pane.w, ctx.tokens.row);
    row(ctx, rect, pad, text, style);
}
