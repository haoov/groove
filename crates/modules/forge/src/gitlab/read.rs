//! What one GitLab reply says, in the same shapes GitHub's reads answer with.

use groove_types::{
    CiState, CiStatus, Forge, MrApproval, MrDetails, MrNote, MrState, MrThread, NotePosition,
    ReviewMr, ReviewState, Reviewer, Timestamp,
};

use crate::Snapshot;

/// One MR out of the reply, or nothing where the query found none.
pub(super) fn snapshot(mr: &serde_json::Value, me: &str, host: &str) -> Option<Snapshot> {
    let iid = mr["iid"].as_str()?;
    Some(Snapshot {
        node: text(&mr["id"]),
        head: text(&mr["diffHeadSha"]),
        number: iid.to_string(),
        details: details(mr, me),
        ci: ci(mr, host),
        threads: threads(mr),
    })
}

fn details(mr: &serde_json::Value, me: &str) -> MrDetails {
    let approved_by = logins(&mr["approvedBy"]);
    MrDetails {
        title: text(&mr["title"]),
        description: text(&mr["description"]),
        author: text(&mr["author"]["username"]),
        source_branch: text(&mr["sourceBranch"]),
        target_branch: text(&mr["targetBranch"]),
        state: state(&mr["state"]),
        draft: mr["draft"].as_bool().unwrap_or_default(),
        created_at: at(&mr["createdAt"]).unwrap_or_default(),
        updated_at: at(&mr["updatedAt"]).unwrap_or_default(),
        web_url: text(&mr["webUrl"]),
        approval: Some(MrApproval {
            approved: mr["approved"].as_bool().unwrap_or(!approved_by.is_empty()),
            approved_by_me: approved_by.iter().any(|name| name == me),
            approved_by,
        }),
        reviewers: reviewers(mr),
    }
}

/// GitLab's own word for the state; one it does not name is open.
fn state(value: &serde_json::Value) -> MrState {
    match value.as_str() {
        Some("merged") => MrState::Merged,
        Some("closed") => MrState::Closed,
        _ => MrState::Open,
    }
}

/// Every reviewer's verdict, timed by the MR's own last change: it has no time here.
fn reviewers(mr: &serde_json::Value) -> Vec<Reviewer> {
    let when = at(&mr["updatedAt"]);
    nodes(&mr["reviewers"])
        .iter()
        .filter_map(|one| {
            let state = verdict(one["mergeRequestInteraction"]["reviewState"].as_str()?)?;
            Some(Reviewer {
                name: text(&one["username"]),
                state,
                at: when,
            })
        })
        .collect()
}

/// What a reviewer's own state means. One who has not looked is still asked.
fn verdict(state: &str) -> Option<ReviewState> {
    match state {
        "APPROVED" => Some(ReviewState::Approved),
        "REQUESTED_CHANGES" => Some(ReviewState::ChangesRequested),
        "REVIEWED" => Some(ReviewState::Commented),
        "UNREVIEWED" | "REVIEW_STARTED" | "UNAPPROVED" => Some(ReviewState::Requested),
        _ => None,
    }
}

/// The pipeline on the MR's head, if it has one.
fn ci(mr: &serde_json::Value, host: &str) -> Option<CiStatus> {
    let pipeline = &mr["headPipeline"];
    let status = pipeline["status"].as_str()?;
    Some(CiStatus {
        state: run(status),
        url: page(host, &text(&pipeline["path"])),
        finished_at: at(&pipeline["finishedAt"]),
    })
}

/// What one of GitLab's pipeline states means for a row.
fn run(status: &str) -> CiState {
    match status {
        "SUCCESS" => CiState::Success,
        "FAILED" => CiState::Failed,
        "RUNNING" | "CANCELING" => CiState::Running,
        "CREATED"
        | "PENDING"
        | "PREPARING"
        | "SCHEDULED"
        | "WAITING_FOR_RESOURCE"
        | "WAITING_FOR_CALLBACK" => CiState::Pending,
        "CANCELED" => CiState::Canceled,
        "SKIPPED" | "MANUAL" => CiState::Skipped,
        _ => CiState::Unknown,
    }
}

/// Every discussion as a thread. One with no note is not one.
fn threads(mr: &serde_json::Value) -> Vec<MrThread> {
    nodes(&mr["discussions"])
        .iter()
        .map(|one| MrThread {
            id: text(&one["id"]),
            notes: notes(one),
        })
        .filter(|thread| !thread.notes.is_empty())
        .collect()
}

fn notes(discussion: &serde_json::Value) -> Vec<MrNote> {
    nodes(&discussion["notes"])
        .iter()
        .map(|note| MrNote {
            author: text(&note["author"]["username"]),
            body: text(&note["body"]),
            created_at: at(&note["createdAt"]).unwrap_or_default(),
            resolved: note["resolved"].as_bool().unwrap_or_default(),
            resolvable: note["resolvable"].as_bool().unwrap_or_default(),
            position: position(&note["position"]),
        })
        .collect()
}

/// Where a note sits in the diff, when it sits anywhere.
fn position(at: &serde_json::Value) -> Option<NotePosition> {
    if at.is_null() {
        return None;
    }
    Some(NotePosition {
        new_path: some(&at["newPath"]),
        new_line: line(&at["newLine"]),
        old_path: some(&at["oldPath"]),
        old_line: line(&at["oldLine"]),
        end_new_line: None,
    })
}

/// One row of the review queue.
pub(super) fn asked(mr: &serde_json::Value) -> Option<ReviewMr> {
    let iid: u64 = mr["iid"].as_str()?.parse().ok()?;
    let project = text(&mr["project"]["fullPath"]);
    if project.is_empty() {
        return None;
    }
    Some(ReviewMr {
        forge: Forge::Gitlab,
        project,
        iid,
        title: text(&mr["title"]),
        author: text(&mr["author"]["username"]),
        source_branch: text(&mr["sourceBranch"]),
        target_branch: text(&mr["targetBranch"]),
        draft: mr["draft"].as_bool().unwrap_or_default(),
        web_url: text(&mr["webUrl"]),
        updated_at: at(&mr["updatedAt"]).unwrap_or_default(),
        local_path: None,
        approved: mr["approved"].as_bool().unwrap_or_default(),
    })
}

/// A page of the host, from the path GitLab gives.
fn page(host: &str, path: &str) -> String {
    match (path.is_empty(), host.starts_with("http")) {
        (true, _) => String::new(),
        (false, true) => format!("{host}{path}"),
        (false, false) => format!("https://{host}{path}"),
    }
}

pub(super) fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

fn some(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

fn line(value: &serde_json::Value) -> Option<u32> {
    let line = value.as_i64().or_else(|| value.as_str()?.parse().ok())?;
    u32::try_from(line).ok()
}

fn at(value: &serde_json::Value) -> Option<Timestamp> {
    Timestamp::parse(value.as_str()?).ok()
}

pub(super) fn nodes(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list["nodes"].as_array().cloned().unwrap_or_default()
}

fn logins(list: &serde_json::Value) -> Vec<String> {
    nodes(list)
        .iter()
        .map(|one| text(&one["username"]))
        .filter(|name| !name.is_empty())
        .collect()
}
