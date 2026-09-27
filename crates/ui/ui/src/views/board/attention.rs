//! Why an item needs the user, in its own words under the title.

use groove_gfx::Rect;
use groove_types::{Attention, Timestamp};

use crate::base::ctx::Ctx;
use crate::base::style::Role;
use crate::text::{ago, row};

/// How long a fact has stood.
fn old(since: Timestamp, now: Timestamp) -> String {
    let seconds = (now.seconds() - since.seconds()).max(0);
    ago(std::time::Duration::from_secs(seconds as u64))
}

/// The reasons an item carries, as the line under its title.
pub(super) fn line(reasons: &[Attention], now: Timestamp) -> String {
    reasons
        .iter()
        .map(|one| words(*one, now))
        .collect::<Vec<String>>()
        .join(" · ")
}

/// One reason, and how long it has stood.
fn words(reason: Attention, now: Timestamp) -> String {
    match reason {
        Attention::ReviewWaiting { since } => format!("waiting {}", old(since, now)),
        Attention::ChangesRequested { since } => {
            format!("changes requested · {}", old(since, now))
        }
        Attention::CiFailed { since } => match since {
            Some(since) => format!("CI failed · {}", old(since, now)),
            None => "CI failed".to_string(),
        },
        Attention::ApprovedUnmerged { since } => format!("approved · {}", old(since, now)),
        Attention::DueSoon { in_days } => match in_days {
            0 => "due today".to_string(),
            days => format!("due in {days}d"),
        },
        Attention::Overdue { by_days } => format!("overdue {by_days}d"),
    }
}

/// The line itself, under the title it belongs to.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, at: f32, text: &str) {
    let style = ctx.styles.small(Role::Attention);
    row(ctx, line, at, text, style);
}
