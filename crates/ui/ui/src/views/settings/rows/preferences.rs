//! The Preferences section's rows: counts that step up and down.

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;
use groove_types::Preferences;

use super::{Row, Section, Value, grouped};

pub(super) fn preferences(app: &AppState) -> Vec<Row> {
    let held = app.config.preferences();
    let mut out = grouped("Attention thresholds", thresholds(&held));
    out.extend(grouped("Forge poll", poll(&held)));
    out
}

/// A count stepped by `step`, never under `least`; `set` makes the preference of a value.
pub(super) fn count<T>(
    label: &'static str,
    words: &'static str,
    (at, step, least): (T, T, T),
    unit: &str,
    set: impl Fn(T) -> Preference,
) -> Row
where
    T: Copy
        + PartialOrd
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::fmt::Display,
{
    let less = (at >= least + step).then(|| set(at - step));
    Row {
        section: Section::Preferences,
        group: "",
        label,
        words,
        value: Value::Count {
            shown: format!("{at} {unit}"),
            less,
            more: set(at + step),
        },
    }
}

fn thresholds(held: &Preferences) -> Vec<Row> {
    let t = held.thresholds;
    let words = "attention threshold";
    vec![
        count(
            "review waiting",
            words,
            (t.review_waiting_days, 1, 0),
            "days",
            Preference::ReviewWaitingDays,
        ),
        count(
            "due soon",
            words,
            (t.due_soon_days, 1, 0),
            "days",
            Preference::DueSoonDays,
        ),
        count(
            "approved unmerged",
            words,
            (t.approved_unmerged_days, 1, 0),
            "days",
            Preference::ApprovedUnmergedDays,
        ),
        count(
            "failed run",
            words,
            (t.ci_failed_minutes, 5, 0),
            "minutes",
            Preference::CiFailedMinutes,
        ),
    ]
}

fn poll(held: &Preferences) -> Vec<Row> {
    let words = "forge poll mr ci";
    vec![
        count(
            "poll interval",
            words,
            (held.poll_interval_secs, 10, 10),
            "seconds",
            Preference::PollIntervalSecs,
        ),
        count(
            "stale after",
            words,
            (held.stale_after_secs, 60, 0),
            "seconds",
            Preference::StaleAfterSecs,
        ),
    ]
}
