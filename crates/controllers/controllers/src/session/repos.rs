//! The session's repos and worktrees: adding them, selecting one, closing them.

use groove_session_service::Added;
use groove_types::{Error, RepoId, SessionId, WorktreeId, WorktreeSpec};

use super::rail::persist_selection;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

pub fn add_repo(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    name: &str,
    spec: WorktreeSpec,
    asker: Asker,
) {
    let Some(session) = state.session.get(id).map(|o| o.session.clone()) else {
        return asker.refused(crate::tools::NO_SESSION);
    };
    let verb = if name.contains("://") || name.contains('@') {
        "cloning"
    } else {
        "adding"
    };
    let pending = state.begin(format!("{verb} {name}"));
    let (service, name) = (services.session.clone(), name.to_string());
    added(spawner, pending, asker, async move {
        service.add_repo(&session, &name, &spec, None).await
    });
}

pub fn add_worktree(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    repo: &RepoId,
    spec: WorktreeSpec,
    asker: Asker,
) {
    let Some(session) = state.session.get(id).map(|o| o.session.clone()) else {
        return asker.refused(crate::tools::NO_SESSION);
    };
    let branch = spec.branch.clone().unwrap_or_default();
    let pending = state.begin(format!("adding worktree {branch}"));
    let (service, repo) = (services.session.clone(), repo.clone());
    added(spawner, pending, asker, async move {
        service.add_worktree(&session, &repo, &spec, None).await
    });
}

pub fn remove_repo(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    repo: &RepoId,
    force: bool,
) {
    let pending = state.begin(format!("removing {repo}"));
    let (service, at, gone) = (services.session.clone(), id.clone(), repo.clone());
    let work = async move { service.remove_repo(&at, &gone, force).await };
    let repo = repo.clone();
    taken(spawner, pending, id, work, move |open, ()| {
        open.remove_repo(&repo)
    });
}

pub fn select_worktree(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    worktree: &WorktreeId,
) {
    let Some(open) = state.session.get_mut(id) else {
        return;
    };
    if !open.worktrees.iter().any(|w| &w.id == worktree) {
        return;
    }
    open.state.selected_worktree = Some(worktree.clone());
    persist_selection(state, services, spawner, id);
    crate::workspace::load(state, spawner);
}

pub fn close_worktree(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    worktree: &WorktreeId,
    force: bool,
) {
    let pending = state.begin("closing worktree");
    let (service, worktree) = (services.session.clone(), worktree.clone());
    let work = async move { service.close_worktree(&worktree, force).await };
    taken(spawner, pending, id, work, |open, closed| {
        open.remove_worktree(&closed.id)
    });
}

/// What the disk let go of, off the session's row; the selection and the workspace follow.
fn taken<T: Send + 'static>(
    spawner: &dyn Spawner,
    pending: u64,
    id: &SessionId,
    work: impl Future<Output = Result<T, Error>> + Send + 'static,
    apply: impl FnOnce(&mut groove_session_service::Open, T) + Send + 'static,
) {
    let id = id.clone();
    spawner.spawn(Box::pin(async move {
        let result = work.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                let done = match result {
                    Ok(done) => done,
                    Err(e) => return state.failed(e),
                };
                if let Some(open) = state.session.get_mut(&id) {
                    let before: Vec<WorktreeId> =
                        open.worktrees.iter().map(|w| w.id.clone()).collect();
                    apply(open, done);
                    let kept: Vec<&WorktreeId> = open.worktrees.iter().map(|w| &w.id).collect();
                    for gone in before.iter().filter(|one| !kept.contains(one)) {
                        state.workspace.forget(gone);
                    }
                }
                persist_selection(state, services, spawner, &id);
                crate::workspace::follow(state, spawner);
            },
        ) as Continuation
    }));
}

pub fn list_repos(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let pool = service.list_pool();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.session.pool = pool
        }) as Continuation
    }));
}

pub fn list_branches(services: &Services, spawner: &dyn Spawner, repo: &RepoId) {
    let (service, repo) = (services.session.clone(), repo.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.list_branches(&repo).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match result {
                Ok(heads) => {
                    state.session.branches.retain(|(r, _)| r != &repo);
                    state.session.branches.push((repo, heads));
                }
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

/// A provisioning result into the session's row, and the board read again.
pub(crate) fn added(
    spawner: &dyn Spawner,
    pending: u64,
    asker: Asker,
    work: impl Future<Output = Result<Added, Error>> + Send + 'static,
) {
    spawner.spawn(Box::pin(async move {
        let result = work.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                let added = match result {
                    Ok(added) => added,
                    Err(e) => return asker.failed(state, e),
                };
                let id = added.worktree.session.clone();
                let said = format!(
                    "{} on {} at {} · worktree_id {}",
                    added.worktree.branch,
                    added.repo.project,
                    added.worktree.path,
                    added.worktree.id.as_str()
                );
                let (kind, branch) = (
                    groove_types::TimelineKind::WorktreeAdded,
                    &added.worktree.branch,
                );
                crate::timeline::log(services, spawner, &id, kind, branch, &added.worktree.id);
                for one in added.notes {
                    state.say(one);
                }
                if let Some(open) = state.session.get_mut(&id) {
                    open.add_worktree(added.repo, added.worktree);
                }
                persist_selection(state, services, spawner, &id);
                crate::workspace::follow(state, spawner);
                super::rail::list(services, spawner);
                asker.done(|| said);
            },
        ) as Continuation
    }));
}
