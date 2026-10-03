//! What a worktree's MR looks like in one line: its number, its checks, its notes.

use groove_gfx::Rect;
use groove_types::{CiState, MrState, ReviewState, WorktreeDelivery};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{Badge, counts};

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
        groove_ui_kit::shape::top_rule(ctx, rule, style.color);
    }
    ctx.hit(link, target);
    let mut at = x + width + ctx.tokens.sm;
    if let Some((word, role)) = ended(mr.state) {
        at = word_at(ctx, line, at, word, role);
    } else {
        at = verdict(ctx, line, at, delivery);
        at = checks(ctx, line, at, delivery);
    }
    counts(ctx, line, at, &[(Mark::Note, delivery.notes, Role::Faint)])
}

/// The room `delivered` needs, for a caller placing it against a right edge.
pub fn room_for(ctx: &mut Ctx, delivery: &WorktreeDelivery) -> f32 {
    let Some(mr) = delivery.mr.as_ref() else {
        return 0.0;
    };
    let style = ctx.styles.small(Role::Muted);
    let mut wide = ctx.measure(&mr.named(), &style) + ctx.tokens.sm;
    let words = match ended(mr.state) {
        Some((word, _)) => [Some(word), None],
        None => [
            mr.review.map(review_said).map(|(word, _)| word),
            delivery.ci.map(|_| CI),
        ],
    };
    for word in words.into_iter().flatten() {
        wide += Badge::new(word, Role::Muted).width(ctx) + ctx.tokens.sm;
    }
    if delivery.notes > 0 {
        wide += style.size + ctx.tokens.xs + ctx.measure(&delivery.notes.to_string(), &style);
        wide += ctx.tokens.md;
    }
    wide
}

const CI: &str = "CI";

/// Where the reviewers stand, as a word in its colour.
fn verdict(ctx: &mut Ctx, line: Rect, x: f32, delivery: &WorktreeDelivery) -> f32 {
    match delivery.mr.as_ref().and_then(|mr| mr.review) {
        Some(review) => review_word(ctx, line, x, review),
        None => x,
    }
}

/// Where the reviewers stand, from `x`. Returns the x after it.
fn review_word(ctx: &mut Ctx, line: Rect, x: f32, review: ReviewState) -> f32 {
    let (word, role) = review_said(review);
    word_at(ctx, line, x, word, role)
}

pub fn review_said(review: ReviewState) -> (&'static str, Role) {
    match review {
        ReviewState::Approved => ("approved", Role::Ok),
        ReviewState::Commented => ("comments", Role::Attention),
        ReviewState::ChangesRequested => ("changes", Role::Bad),
        ReviewState::Requested => ("review", Role::Attention),
    }
}

/// The run on the branch's head, as `CI` in its state's colour.
fn checks(ctx: &mut Ctx, line: Rect, x: f32, delivery: &WorktreeDelivery) -> f32 {
    let Some(ci) = delivery.ci else {
        return x;
    };
    let role = match ci {
        CiState::Success => Role::Ok,
        CiState::Failed => Role::Bad,
        CiState::Running | CiState::Pending => Role::Attention,
        CiState::Canceled | CiState::Skipped | CiState::Unknown => Role::Ghost,
    };
    word_at(ctx, line, x, CI, role)
}

/// Returns the x after the badge.
fn word_at(ctx: &mut Ctx, line: Rect, x: f32, word: &str, role: Role) -> f32 {
    Badge::new(word, role).at(ctx, line, x).right() + ctx.tokens.sm
}

/// The colour an MR's own state carries.
fn of_state(state: MrState) -> Role {
    match state {
        MrState::Open => Role::Working,
        MrState::Merged => Role::Merged,
        MrState::Closed => Role::Ghost,
    }
}

/// The badge an MR that is no longer open carries in place of its review and its run.
fn ended(state: MrState) -> Option<(&'static str, Role)> {
    match state {
        MrState::Open => None,
        MrState::Merged => Some(("merged", Role::Merged)),
        MrState::Closed => Some(("closed", Role::Ghost)),
    }
}
