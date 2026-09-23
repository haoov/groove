//! The session's repos and worktrees: adding them, selecting one, closing them.

use groove_session_service::Added;
use groove_types::{Error, RepoId, SessionId, WorktreeId, WorktreeSpec};

use super::rail::persist_selection;
use crate::{AppState, Continuation, Services, Spawner};

pub fn add_repo(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    name: &str,
    spec: WorktreeSpec,
) {
    let Some(session) = state.session.get(id).map(|o| o.session.clone()) else {
        return;
    };
    let verb = if name.contains("://") || name.contains('@') {
        "cloning"
    } else {
        "adding"
    };
    let pending = state.begin(format!("{verb} {name}"));
    let (service, name) = (services.session.clone(), name.to_string());
    added(spawner, pending, async move {
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
) {
    let Some(session) = state.session.get(id).map(|o| o.session.clone()) else {
        return;
    };
    let branch = spec.branch.clone().unwrap_or_default();
    let pending = state.begin(format!("adding worktree {branch}"));
    let (service, repo) = (services.session.clone(), repo.clone());
    added(spawner, pending, async move {
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
    let (service, id, repo) = (services.session.clone(), id.clone(), repo.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.remove_repo(&id, &repo, force).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                match result {
                    Ok(()) => {
                        if let Some(open) = state.session.get_mut(&id) {
                            open.remove_repo(&repo);
                        }
                        persist_selection(state, services, spawner, &id);
                        crate::workspace::follow(state, spawner);
                    }
                    Err(e) => state.failed(e),
                }
            },
        ) as Continuation
    }));
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
    let (service, id, worktree) = (services.session.clone(), id.clone(), worktree.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.close_worktree(&worktree, force).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                match result {
                    Ok(closed) => {
                        if let Some(open) = state.session.get_mut(&id) {
                            open.remove_worktree(&closed.id);
                        }
                        persist_selection(state, services, spawner, &id);
                        crate::workspace::follow(state, spawner);
                    }
                    Err(e) => state.failed(e),
                }
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

/// The line a provisioning leaves: the worktree it made, and where it stands.
fn logged_added(
    services: &Services,
    spawner: &dyn Spawner,
    session: &groove_types::SessionId,
    added: &Added,
) {
    crate::timeline::logged(
        services,
        spawner,
        session.clone(),
        groove_types::TimelineKind::WorktreeAdded,
        added.worktree.branch.clone(),
        serde_json::json!({ "worktree": added.worktree.id.as_str() }),
    );
}

/// A provisioning result into the session's row: the repo, the worktree, the notes.
fn added(
    spawner: &dyn Spawner,
    pending: u64,
    work: impl Future<Output = Result<Added, Error>> + Send + 'static,
) {
    spawner.spawn(Box::pin(async move {
        let result = work.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                match result {
                    Ok(added) => {
                        let id = added.worktree.session.clone();
                        logged_added(services, spawner, &id, &added);
                        for said in added.notes {
                            state.say(said);
                        }
                        if let Some(open) = state.session.get_mut(&id) {
                            open.add_worktree(added.repo, added.worktree);
                        }
                        persist_selection(state, services, spawner, &id);
                        crate::workspace::follow(state, spawner);
                    }
                    Err(e) => state.failed(e),
                }
            },
        ) as Continuation
    }));
}
