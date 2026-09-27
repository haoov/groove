//! The agent's row under its screen: auto-approve, the skills, and a reload.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;

use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::box_in;
use crate::text::row;
use crate::widgets::slot_at;

/// What the agent waits on, or what it can be sent.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let line = ctx.layout.agent_bar;
    ctx.quad(line, ctx.styles.band());
    let rule = Rect::new(line.x, line.y, line.w, ctx.tokens.hairline);
    ctx.quad(rule, ctx.styles.line());
    offered(ctx, line, app, open);
}

/// The reload, the skills menu and the auto-approve switch, from the row's own end.
fn offered(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let id = open.session.id.clone();
    let stale = app.agent.stale(&id);
    let auto = app.agent.activity(&id).is_some_and(|one| one.auto_approve);
    let mut left = line.right();
    for (label, target, role, caret) in [
        (
            "reload",
            Target::Reload(id.clone()),
            reload_role(stale),
            false,
        ),
        ("skills", Target::Skills(id.clone()), Role::Muted, true),
        (
            switched(auto),
            Target::AutoApprove(id.clone()),
            auto_role(auto),
            false,
        ),
    ] {
        let hovered = ctx.hovered(&target);
        let style = ctx.styles.small(match hovered {
            true => Role::Text,
            false => role,
        });
        let word = ctx.measure(label, &style);
        let held = match caret {
            true => word + ctx.tokens.xs + ctx.tokens.small,
            false => word,
        };
        let room = Rect::new(line.x, line.y, left - line.x, line.h);
        let at = left - held - ctx.tokens.sm * 2.0 - ctx.tokens.xs;
        let box_ = slot_at(ctx, room, at, held, Some(ctx.styles.ground()));
        row(ctx, box_, ctx.tokens.sm, label, style);
        if caret {
            let size = ctx.tokens.small;
            let mark = box_in(box_, box_.right() - ctx.tokens.sm - size, size);
            ctx.icon(mark, Mark::Down, Mark::UPWARDS, style.color);
        }
        ctx.hit(box_, target);
        left = box_.x - ctx.tokens.xs;
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
