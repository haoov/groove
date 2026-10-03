//! The checks on the last commit, reduced to one state for the row.

use groove_types::{CiState, CiStatus, Timestamp};

use super::reviews::nodes;
use super::{at, text};

/// One check, as either kind of context reports itself.
struct Check {
    state: CiState,
    url: String,
    at: Option<Timestamp>,
}

/// The run as the check that decides it: the worst state, its page, its own time.
pub(super) fn status(pr: &serde_json::Value) -> Option<CiStatus> {
    let rollup = &pr["commits"]["nodes"][0]["commit"]["statusCheckRollup"];
    let checks: Vec<Check> = nodes(&rollup["contexts"]).iter().map(check).collect();
    let state = checks.iter().map(|one| one.state).reduce(CiState::worst)?;
    let shown = checks.iter().find(|one| one.state == state)?;
    Some(CiStatus {
        state,
        url: match shown.url.is_empty() {
            true => text(&pr["url"]),
            false => shown.url.clone(),
        },
        finished_at: shown.at,
    })
}

/// A check run reports a status and a conclusion; a commit status reports a state.
fn check(context: &serde_json::Value) -> Check {
    let conclusion = context["conclusion"].as_str();
    let state = match context["status"].as_str() {
        Some(status) => of_check_run(status, conclusion),
        None => of_commit_status(context["state"].as_str().unwrap_or_default()),
    };
    Check {
        state,
        url: match text(&context["detailsUrl"]) {
            url if url.is_empty() => text(&context["targetUrl"]),
            url => url,
        },
        at: at(&context["completedAt"]).or_else(|| at(&context["createdAt"])),
    }
}

fn of_check_run(status: &str, conclusion: Option<&str>) -> CiState {
    match status {
        "QUEUED" | "WAITING" | "REQUESTED" | "PENDING" => CiState::Pending,
        "IN_PROGRESS" => CiState::Running,
        _ => match conclusion.unwrap_or_default() {
            "SUCCESS" | "NEUTRAL" => CiState::Success,
            "SKIPPED" => CiState::Skipped,
            "CANCELLED" => CiState::Canceled,
            "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => CiState::Failed,
            _ => CiState::Unknown,
        },
    }
}

fn of_commit_status(state: &str) -> CiState {
    match state {
        "SUCCESS" => CiState::Success,
        "PENDING" => CiState::Pending,
        "FAILURE" | "ERROR" => CiState::Failed,
        _ => CiState::Unknown,
    }
}
