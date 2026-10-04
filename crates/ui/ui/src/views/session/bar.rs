//! The agent's row under its screen: auto-approve, the skills, and a reload.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_gfx::Edges;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::box_in;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{Button, Toggle};

/// What the agent waits on, or what it can be sent.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let line = ctx.app.layout.agent_bar;
    groove_ui_kit::shape::ground(ctx, line, Ground::Band);
    let (thick, rule) = (ctx.tokens.hairline, ctx.styles.line());
    groove_ui_kit::shape::top_rule(ctx, line, rule);
    groove_ui_kit::shape::side_rule(ctx, line, line.right() - thick, rule);
    offered(ctx, line, app, open);
}

/// The reload, the skills menu and the auto-approve switch, from the row's own end.
fn offered(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let id = open.session.id.clone();
    let stale = app.agent.stale(&id);
    let auto = open.state.auto_approve;
    let (ground, xs) = (ctx.styles.ground(), ctx.tokens.xs);
    let mut room = line.pad(Edges::across(0.0, xs));
    let words = [
        Button::new(
            "reload",
            Target::Reload(id.clone()),
            reload_role(stale),
            ground,
        ),
        Button::new("skills", Target::Skills(id.clone()), Role::Muted, ground).caret(Mark::UPWARDS),
    ];
    for word in words {
        word.lit(Role::Text).right(ctx, &mut room, xs * 2.0);
    }
    let auto_approve = Toggle::new(auto, Target::AutoApprove(id.clone())).label("auto-approve");
    auto_approve
        .role(Role::Attention)
        .right(ctx, &mut room, xs * 2.0);
    said(ctx, line, app, open);
}

fn counted(skills: usize) -> String {
    match skills {
        1 => "1 skill".to_string(),
        n => format!("{n} skills"),
    }
}

fn reload_role(stale: bool) -> Role {
    match stale {
        true => Role::Attention,
        false => Role::Muted,
    }
}

/// What the row says when nothing waits: the agent's own name for itself.
fn said(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let size = ctx.tokens.small;
    let box_ = box_in(line, line.x + ctx.tokens.md, size);
    groove_ui_kit::widgets::icon(ctx, box_, Mark::Busy, Role::Faint);
    let style = ctx.styles.small(Role::Faint);
    let at = ctx.tokens.md + size + ctx.tokens.xs;
    let count = counted(app.agent.skills_for(&open.session.kind).len());
    row(ctx, line, at, &count, style);
}
