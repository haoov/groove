//! The branch's history: the commits it holds, and one of them shown as a change.

use groove_session_service::names;
use groove_types::CommitEntry;
use groove_workspace_service::{
    COMMITS_MAX, Revision, at_commit, commits, revision as revision_at,
};

use crate::{AppState, Continuation, Services, Spawner};

/// The newest commits of the selected worktree's branch.
pub(super) fn list(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(asked) = super::Asked::now(state) else {
        state.workspace.logged = None;
        return state.workspace.log.clear();
    };
    state.workspace.logged = Some(asked.worktree.clone());
    let job = state.begin("reading the commits");
    spawner.spawn(Box::pin(async move {
        let read = commits(&asked.dir, asked.base.as_deref(), COMMITS_MAX).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            if !asked.holds(state) {
                return;
            }
            match read {
                Err(e) => state.failed(e),
                Ok(log) => state.workspace.log = log,
            }
        }) as Continuation
    }));
}

/// One commit of the list or of a blame shown as the change it made, which nothing may write.
pub(super) fn open(state: &mut AppState, spawner: &dyn Spawner, sha: String) {
    let Some(entry) = known(state, &sha) else {
        return;
    };
    read(state, spawner, Some(entry), (sha, false));
}

fn known(state: &AppState, sha: &str) -> Option<CommitEntry> {
    let listed = state.workspace.log.iter().find(|one| one.sha == sha);
    listed
        .cloned()
        .or_else(|| state.workspace.blames.entry(sha))
}

/// `sha` read from git when the list does not hold it; the last one asked wins, and a revision's read ends it.
fn read(
    state: &mut AppState,
    spawner: &dyn Spawner,
    entry: Option<CommitEntry>,
    (sha, revision): (String, bool),
) {
    let Some(asked) = super::Asked::now(state) else {
        return;
    };
    state.workspace.opening += 1;
    let nth = state.workspace.opening;
    let short = groove_types::short_sha(&sha);
    let job = state.begin(format!("reading {short}"));
    spawner.spawn(Box::pin(async move {
        let entry = match entry {
            Some(entry) => Ok(entry),
            None => revision_at(&asked.dir, &sha).await,
        };
        let read = match entry {
            Ok(entry) => at_commit(&asked.dir, &sha).await.map(|read| (entry, read)),
            Err(e) => Err(e),
        };
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            if revision {
                state.workspace.revision = None;
            }
            if state.workspace.opening != nth || !asked.holds_place(state) {
                return;
            }
            match read {
                Err(e) => state.failed(e),
                Ok((entry, (files, changes))) => shown(state, entry, files, changes),
            }
        }) as Continuation
    }));
}

/// A worktree of the repo `repo` names selected, the repo added from `target` when missing; then `sha` shown.
pub(super) fn revision(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (repo, target): (String, Option<String>),
    sha: Option<String>,
) {
    let Some(open) = state.session.selected() else {
        return;
    };
    let (session, slug) = (open.session.id.clone(), groove_session_service::slug(&repo));
    let held = open.repos.iter().any(|one| names(&one.id, &slug));
    state.workspace.revision = Some(Revision {
        session: session.clone(),
        slug,
        sha,
        reading: false,
    });
    if !held {
        let source = (repo.as_str(), target.as_deref());
        return crate::session::add_repo_at(state, services, spawner, &session, source);
    }
    due(state, services, spawner);
}

/// The repo asked for: a worktree of it selected, then, once read, its commit shown.
pub(crate) fn due(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let Some(pending) = state.workspace.revision.clone().filter(|one| !one.reading) else {
        return;
    };
    let Some(open) = state.session.selected() else {
        return;
    };
    if open.session.id != pending.session {
        state.workspace.revision = None;
        return;
    }
    let ours = |one: &groove_types::Worktree| names(&one.repo, &pending.slug);
    if state.session.selected_worktree().is_some_and(ours) {
        if super::stale(state) {
            return;
        }
        match pending.sha {
            Some(sha) => {
                if let Some(one) = state.workspace.revision.as_mut() {
                    one.reading = true;
                }
                read(state, spawner, known(state, &sha), (sha, true));
            }
            None => state.workspace.revision = None,
        }
        return;
    }
    let session = open.session.id.clone();
    let Some(worktree) = open
        .worktrees
        .iter()
        .find(|one| ours(one))
        .map(|one| one.id.clone())
    else {
        return;
    };
    crate::session::select_worktree(state, services, spawner, &session, &worktree);
}

/// What the commit changed, in place of the working tree.
fn shown(
    state: &mut AppState,
    entry: CommitEntry,
    files: Vec<groove_types::FileDiff>,
    changes: groove_workspace_service::Changes,
) {
    let Some(worktree) = state.session.selected_worktree().map(|one| one.id.clone()) else {
        return;
    };
    state.workspace.loaded(worktree, files, changes);
    state.workspace.commit = Some(entry);
}

/// The working tree again, in place of the commit.
pub(super) fn leave(state: &mut AppState, spawner: &dyn Spawner) {
    if state.workspace.commit.take().is_none() {
        return;
    }
    super::diff::reread(state, spawner);
}
