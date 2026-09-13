use crate::{CiState, Day, MrState, TaskDates, Timestamp};

/// Days, from Config › Preferences.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Thresholds {
    pub review_waiting_days: u32,
    pub due_soon_days: u32,
    pub approved_unmerged_days: u32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            review_waiting_days: 3,
            due_soon_days: 2,
            approved_unmerged_days: 1,
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
    if mr.ci == Some(CiState::Failed) {
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
