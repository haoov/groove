//! The agent's row under its screen: auto-approve, the skills, and a reload.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_gfx::Edges;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::box_in;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::Button;

/// What the agent waits on, or what it can be sent.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let line = ctx.app.layout.agent_bar;
    ctx.quad(line, ctx.styles.band());
    let (thick, rule) = (ctx.tokens.hairline, ctx.styles.line());
    ctx.quad(Rect::new(line.x, line.y, line.w, thick), rule);
    ctx.quad(Rect::new(line.right() - thick, line.y, thick, line.h), rule);
    offered(ctx, line, app, open);
}

/// The reload, the skills menu and the auto-approve switch, from the row's own end.
fn offered(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let id = open.session.id.clone();
    let stale = app.agent.stale(&id);
    let auto = app.agent.activity(&id).is_some_and(|one| one.auto_approve);
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
        Button::new(
            switched(auto),
            Target::AutoApprove(id.clone()),
            auto_role(auto),
            ground,
        ),
    ];
    for word in words {
        word.lit(Role::Text).right(ctx, &mut room, xs * 2.0);
    }
    said(ctx, line, app, open, stale);
}

fn counted(skills: usize) -> String {
    match skills {
        1 => "1 skill".to_string(),
        n => format!("{n} skills"),
    }
}

fn switched(on: bool) -> &'static str {
    match on {
        true => "auto-approve on",
        false => "auto-approve off",
    }
}

fn auto_role(on: bool) -> Role {
    match on {
        true => Role::Attention,
        false => Role::Muted,
    }
}

fn reload_role(stale: bool) -> Role {
    match stale {
        true => Role::Attention,
        false => Role::Muted,
    }
}

/// What the row says when nothing waits: the agent's own name for itself.
fn said(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open, stale: bool) {
    let size = ctx.tokens.small;
    let box_ = box_in(line, line.x + ctx.tokens.md, size);
    ctx.icon(box_, Mark::Busy, 0, ctx.styles.color(Role::Faint));
    let style = ctx.styles.small(Role::Faint);
    let at = ctx.tokens.md + size + ctx.tokens.xs;
    let count = counted(app.agent.skills_for(&open.session.kind).len());
    let text = match stale {
        true => format!("{count} · one is newer than the agent"),
        false => count,
    };
    row(ctx, line, at, &text, style);
}
