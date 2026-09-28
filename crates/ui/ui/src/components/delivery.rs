//! What a worktree's MR looks like in one line: its number, its checks, its notes.

use groove_gfx::Rect;
use groove_types::{CiState, MrState, WorktreeDelivery};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::box_in;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::counts;

/// The MR, its checks and its open notes from `x`; returns the x after them.
pub fn delivered(ctx: &mut Ctx, line: Rect, x: f32, delivery: &WorktreeDelivery) -> f32 {
    let Some(mr) = delivery.mr.as_ref() else {
        return x;
    };
    let role = match delivery.stale {
        true => Role::Ghost,
        false => of_state(mr.state),
    };
    let target = Target::MrPage(mr.url.clone());
    let hovered = ctx.hovered(&target);
    let style = ctx.styles.small(if hovered { Role::Text } else { role });
    let name = mr.named();
    let width = ctx.measure(&name, &style);
    let link = Rect::new(x, line.y, width, line.h);
    row(ctx, link, 0.0, &name, style);
    if hovered {
        let under = link.y + (link.h + style.size) / 2.0;
        let rule = Rect::new(x, under, width, ctx.tokens.hairline);
        ctx.quad(rule, style.color);
    }
    ctx.hit(link, target);
    let mut at = x + width + ctx.tokens.xs;
    at = verdict(ctx, line, at, delivery);
    at = checks(ctx, line, at, delivery);
    counts(ctx, line, at, &[(Mark::Note, delivery.notes, Role::Faint)])
}

/// The room `delivered` needs, for a caller placing it against a right edge.
pub fn room_for(ctx: &mut Ctx, delivery: &WorktreeDelivery) -> f32 {
    let Some(mr) = delivery.mr.as_ref() else {
        return 0.0;
    };
    let style = ctx.styles.small(Role::Muted);
    let mut wide = ctx.measure(&mr.named(), &style) + ctx.tokens.xs;
    if mr.approved || mr.changes_requested {
        wide += style.size + ctx.tokens.xs;
    }
    if delivery.ci.is_some() {
        wide += style.size + ctx.tokens.xs;
    }
    if delivery.notes > 0 {
        wide += style.size + ctx.tokens.xs + ctx.measure(&delivery.notes.to_string(), &style);
        wide += ctx.tokens.md;
    }
    wide
}

/// What the reviewers said, when one of them has said anything.
fn verdict(ctx: &mut Ctx, line: Rect, x: f32, delivery: &WorktreeDelivery) -> f32 {
    let Some(mr) = delivery.mr.as_ref() else {
        return x;
    };
    let (mark, role) = match (mr.changes_requested, mr.approved) {
        (true, _) => (Mark::Failed, Role::Attention),
        (false, true) => (Mark::Read, Role::Ok),
        (false, false) => return x,
    };
    mark_at(ctx, line, x, mark, role)
}

/// The state of the run on the branch's head, when it reported one.
fn checks(ctx: &mut Ctx, line: Rect, x: f32, delivery: &WorktreeDelivery) -> f32 {
    let Some(ci) = delivery.ci else {
        return x;
    };
    let (mark, role) = match ci {
        CiState::Success => (Mark::Read, Role::Ok),
        CiState::Failed => (Mark::Failed, Role::Bad),
        CiState::Running | CiState::Pending => (Mark::Busy, Role::Working),
        CiState::Canceled | CiState::Skipped | CiState::Unknown => (Mark::Busy, Role::Ghost),
    };
    mark_at(ctx, line, x, mark, role)
}

/// One mark in the row's middle. Returns the x after it.
fn mark_at(ctx: &mut Ctx, line: Rect, x: f32, mark: Mark, role: Role) -> f32 {
    let size = ctx.styles.small(role).size;
    let box_ = box_in(line, x, size);
    ctx.icon(box_, mark, 0, ctx.styles.color(role));
    x + size + ctx.tokens.xs
}

/// The colour an MR's own state carries.
fn of_state(state: MrState) -> Role {
    match state {
        MrState::Open => Role::Working,
        MrState::Merged => Role::Accent,
        MrState::Closed => Role::Ghost,
    }
}
