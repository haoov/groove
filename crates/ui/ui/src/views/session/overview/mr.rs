//! What the overview shows of the selected worktree's merge request.

use groove_controllers::workspace_service::{Delivery, Snapshot};
use groove_gfx::Rect;
use groove_types::{MrDetails, ReviewState};

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{Row, elide, list};

const UNSET: &str = "—";

/// The MR's own line, then what it stands at. Returns the y under the last.
pub(super) fn rows(ctx: &mut Ctx, area: Rect, top: f32, delivery: &Delivery) -> f32 {
    let (Some(mr), Some(read)) = (delivery.mr.as_ref(), delivery.read.as_ref()) else {
        return top;
    };
    let (label, value) = (ctx.styles.body(Role::Faint), ctx.styles.body(Role::Text));
    let at = ctx.tokens.aside_near + ctx.tokens.md;
    let details = &read.details;
    let title = elide(
        ctx,
        &details.title,
        &value,
        area.w - at - ctx.tokens.md * 2.0,
    );
    let named = format!("{}{}", mr.forge.sigil(), mr.remote_id);
    let held = [
        (named.as_str(), title),
        ("State", state(details, delivery.stale)),
        ("Into", details.target_branch.clone()),
        ("Author", details.author.clone()),
        ("Checks", checks(read)),
        ("Review", review(details)),
        ("Notes", notes(read)),
    ];
    let rows: Vec<Row<'_>> = held
        .iter()
        .map(|(one, held)| Row::new(ctx.tokens.md, one, label).aside(at, held, value))
        .collect();
    list(
        ctx,
        Rect::new(area.x, top, area.w, ctx.tokens.row),
        &rows,
        None,
    )
}

/// Its state, and whether the last read failed.
fn state(details: &MrDetails, stale: bool) -> String {
    let draft = match details.draft {
        true => " · draft",
        false => "",
    };
    let old = match stale {
        true => " · not read just now",
        false => "",
    };
    format!("{}{draft}{old}", details.state.label())
}

fn checks(read: &Snapshot) -> String {
    read.ci
        .as_ref()
        .map_or_else(|| UNSET.to_string(), |one| one.state.label().to_string())
}

/// What the reviewers have said, or who is still being waited on.
fn review(details: &MrDetails) -> String {
    if details.changes_requested() {
        return format!(
            "changes requested by {}",
            who(details, ReviewState::ChangesRequested)
        );
    }
    let approved = details.approval.as_ref().is_some_and(|one| one.approved);
    if approved {
        return format!("approved by {}", who(details, ReviewState::Approved));
    }
    let waiting = who(details, ReviewState::Requested);
    match waiting.is_empty() {
        true => UNSET.to_string(),
        false => format!("waiting on {waiting}"),
    }
}

/// Everyone whose latest verdict is this one.
fn who(details: &MrDetails, state: ReviewState) -> String {
    let names: Vec<&str> = details
        .reviewers
        .iter()
        .filter(|one| one.state == state)
        .map(|one| one.name.as_str())
        .collect();
    names.join(", ")
}

fn notes(read: &Snapshot) -> String {
    let open = read
        .threads
        .iter()
        .filter(|thread| thread.notes.iter().any(|note| !note.resolved))
        .count();
    match (read.threads.len(), open) {
        (0, _) => UNSET.to_string(),
        (all, 0) => format!("{all} resolved"),
        (all, open) => format!("{open} of {all} open"),
    }
}
