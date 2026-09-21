use crate::{CiState, Day, MrState, TaskDates, Timestamp};

/// How long a fact stands before it asks for the user, from Config › Preferences.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Thresholds {
    pub review_waiting_days: u32,
    pub due_soon_days: u32,
    pub approved_unmerged_days: u32,
    /// A failure is left alone this long, for the push that fixes it.
    pub ci_failed_minutes: u32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            review_waiting_days: 3,
            due_soon_days: 2,
            approved_unmerged_days: 1,
            ci_failed_minutes: 10,
        }
    }
}

/// What the forge says about a task's MR, reduced to what the rules read.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MrFacts {
    pub state: Option<MrState>,
    pub review_requested_at: Option<Timestamp>,
    pub changes_requested_at: Option<Timestamp>,
    pub ci: Option<CiState>,
    pub ci_finished_at: Option<Timestamp>,
    pub approved_at: Option<Timestamp>,
}

impl MrFacts {
    /// Two MRs of one task as one: earliest wait, worst run, approval only if both.
    pub fn and(self, other: Self) -> Self {
        Self {
            state: open_of(self.state, other.state),
            review_requested_at: earliest(self.review_requested_at, other.review_requested_at),
            changes_requested_at: earliest(self.changes_requested_at, other.changes_requested_at),
            ci: match (self.ci, other.ci) {
                (Some(one), Some(two)) => Some(one.worst(two)),
                (one, two) => one.or(two),
            },
            ci_finished_at: latest(self.ci_finished_at, other.ci_finished_at),
            approved_at: match (self.approved_at, other.approved_at) {
                (Some(one), Some(two)) => Some(one.max(two)),
                _ => None,
            },
        }
    }
}

/// A task with one MR still open has an open MR.
fn open_of(one: Option<MrState>, two: Option<MrState>) -> Option<MrState> {
    match (one, two) {
        (Some(MrState::Open), _) | (_, Some(MrState::Open)) => Some(MrState::Open),
        (one, two) => one.or(two),
    }
}

fn earliest(one: Option<Timestamp>, two: Option<Timestamp>) -> Option<Timestamp> {
    match (one, two) {
        (Some(one), Some(two)) => Some(one.min(two)),
        (one, two) => one.or(two),
    }
}

fn latest(one: Option<Timestamp>, two: Option<Timestamp>) -> Option<Timestamp> {
    match (one, two) {
        (Some(one), Some(two)) => Some(one.max(two)),
        (one, two) => one.or(two),
    }
}

/// One reason an item needs the user, with the fact it rests on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attention {
    ReviewWaiting { since: Timestamp },
    ChangesRequested { since: Timestamp },
    CiFailed { since: Option<Timestamp> },
    ApprovedUnmerged { since: Timestamp },
    DueSoon { in_days: i64 },
    Overdue { by_days: i64 },
}

/// The rules, in the order the item shows them.
pub fn attention(
    mr: &MrFacts,
    dates: &TaskDates,
    now: Timestamp,
    t: &Thresholds,
) -> Vec<Attention> {
    let mut out = Vec::new();
    if mr.state == Some(MrState::Open) {
        out.extend(open_mr(mr, now, t));
    }
    out.extend(due(dates, now.day(), t));
    out
}

fn open_mr(mr: &MrFacts, now: Timestamp, t: &Thresholds) -> Vec<Attention> {
    let mut out = Vec::new();
    if let Some(since) = mr.changes_requested_at {
        out.push(Attention::ChangesRequested { since });
    }
    if mr.ci == Some(CiState::Failed) && settled(mr.ci_finished_at, now, t) {
        out.push(Attention::CiFailed {
            since: mr.ci_finished_at,
        });
    }
    if let Some(since) = mr.approved_at {
        let green = mr.ci.is_none_or(CiState::is_green);
        if green && since.days_before(now) >= i64::from(t.approved_unmerged_days) {
            out.push(Attention::ApprovedUnmerged { since });
        }
    }
    if let Some(since) = mr.review_requested_at {
        let waiting = mr.approved_at.is_none() && mr.changes_requested_at.is_none();
        if waiting && since.days_before(now) >= i64::from(t.review_waiting_days) {
            out.push(Attention::ReviewWaiting { since });
        }
    }
    out
}

/// Whether a failure has stood long enough to mention. One with no time to it has.
fn settled(at: Option<Timestamp>, now: Timestamp, t: &Thresholds) -> bool {
    let waited = u64::from(t.ci_failed_minutes) * 60;
    at.is_none_or(|at| at.age_at(now).as_secs() >= waited)
}

fn due(dates: &TaskDates, today: Day, t: &Thresholds) -> Option<Attention> {
    let due = dates.due?;
    let in_days = today.days_until(due);
    if in_days < 0 {
        Some(Attention::Overdue { by_days: -in_days })
    } else if in_days <= i64::from(t.due_soon_days) {
        Some(Attention::DueSoon { in_days })
    } else {
        None
    }
}
