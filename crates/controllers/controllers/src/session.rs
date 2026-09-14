//! The `session` controller: one function per user action on the `session` service.

use groove_session_service::Added;
use groove_types::{Error, RepoId, SessionId, Timestamp, WorktreeId, WorktreeSpec};

use crate::{AppState, Continuation, Services, Spawner, agent};

/// The grid an agent starts on; the pane resizes it on its first frame.
const FIRST_SIZE: (u16, u16) = (80, 24);

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `session.restore`: the rail as it was when the app last closed, agents started.
    Restore,
    /// `session.open_explorer`: a session with no ticket yet, its agent started.
    OpenExplorer { title: Option<String> },
    /// `session.rename_explorer`
    RenameExplorer { session: SessionId, title: String },
    /// `session.discard_explorer`: end the agent, delete the session and what it owns.
    DiscardExplorer { session: SessionId },
    /// `session.select`: make it the current one.
    Select { session: SessionId },
    /// `session.close`: end the agent, drop the row; the session stays on disk.
    Close { session: SessionId },
    /// `session.add_repo`: a pool repo by name, its first worktree cut.
    AddRepo {
        session: SessionId,
        name: String,
        spec: WorktreeSpec,
    },
    /// `session.remove_repo`: close its worktrees, detach it.
    RemoveRepo {
        session: SessionId,
        repo: RepoId,
        force: bool,
    },
    /// `session.add_worktree`: another branch of a repo the session holds.
    AddWorktree {
        session: SessionId,
        repo: RepoId,
        spec: WorktreeSpec,
    },
    /// `session.select_worktree`: the one the workspace follows.
    SelectWorktree {
        session: SessionId,
        worktree: WorktreeId,
    },
    /// `session.close_worktree`: the directory goes, the branch stays.
    CloseWorktree {
        session: SessionId,
        worktree: WorktreeId,
        force: bool,
    },
    /// `session.list_repos`: refresh the pool listing for the pickers.
    ListRepos,
    /// `session.list_branches`: refresh origin's heads of a repo for the pickers.
    ListBranches { repo: RepoId },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Restore => "session.restore",
            Command::OpenExplorer { .. } => "session.open_explorer",
            Command::RenameExplorer { .. } => "session.rename_explorer",
            Command::DiscardExplorer { .. } => "session.discard_explorer",
            Command::Select { .. } => "session.select",
            Command::Close { .. } => "session.close",
            Command::AddRepo { .. } => "session.add_repo",
            Command::RemoveRepo { .. } => "session.remove_repo",
            Command::AddWorktree { .. } => "session.add_worktree",
            Command::SelectWorktree { .. } => "session.select_worktree",
            Command::CloseWorktree { .. } => "session.close_worktree",
            Command::ListRepos => "session.list_repos",
            Command::ListBranches { .. } => "session.list_branches",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Restore => restore(services, spawner),
        Command::OpenExplorer { title } => {
            open_explorer(state, services, spawner, title.as_deref())
        }
        Command::RenameExplorer { session, title } => {
            rename_explorer(state, services, spawner, &session, &title)
        }
        Command::DiscardExplorer { session } => {
            discard_explorer(state, services, spawner, &session)
        }
        Command::Select { session } => select(state, services, spawner, &session),
        Command::Close { session } => close(state, services, spawner, &session),
        Command::AddRepo {
            session,
            name,
            spec,
        } => add_repo(state, services, spawner, &session, &name, spec),
        Command::RemoveRepo {
            session,
            repo,
            force,
        } => remove_repo(services, spawner, &session, &repo, force),
        Command::AddWorktree {
            session,
            repo,
            spec,
        } => add_worktree(state, services, spawner, &session, &repo, spec),
        Command::SelectWorktree { session, worktree } => {
            select_worktree(state, services, spawner, &session, &worktree)
        }
        Command::CloseWorktree {
            session,
            worktree,
            force,
        } => close_worktree(services, spawner, &session, &worktree, force),
        Command::ListRepos => list_repos(services, spawner),
        Command::ListBranches { repo } => list_branches(services, spawner, &repo),
    }
}

/// Reads the opened rows; the continuation puts them on the rail, loads each, starts each agent.
pub fn restore(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let result = service.opened().await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                let rows = match result {
                    Ok(rows) => rows,
                    Err(e) => return state.errors.push(e),
                };
                let last_seen = rows
                    .iter()
                    .max_by_key(|(_, st)| st.seen_at)
                    .map(|(s, _)| s.id.clone());
                for (session, session_state) in rows {
                    let id = session.id.clone();
                    state.session.restore(session, session_state);
                    load_contents(services, spawner, &id);
                    agent::start(state, spawner, id, FIRST_SIZE);
                }
                state.session.selected = last_seen;
            },
        ) as Continuation
    }));
}

/// The session's recorded repos and worktrees into its row.
fn load_contents(services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let (service, id) = (services.session.clone(), id.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.contents(&id).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match result {
                Ok((repos, worktrees)) => {
                    if let Some(open) = state.session.get_mut(&id) {
                        open.repos = repos;
                        open.worktrees = worktrees;
                        let still_there = open.selected_worktree().is_some();
                        if !still_there {
                            open.state.selected_worktree =
                                open.worktrees.first().map(|w| w.id.clone());
                        }
                    }
                }
                Err(e) => state.errors.push(e),
            },
        ) as Continuation
    }));
}

pub fn open_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    title: Option<&str>,
) {
    let now = Timestamp::now();
    let session = groove_session_service::explorer(title, now);
    let id = session.id.clone();
    state.session.open(session.clone(), now);
    agent::start(state, spawner, id, FIRST_SIZE);
    let service = services.session.clone();
    record(spawner, async move {
        service.create_explorer(&session, now).await
    });
}

pub fn rename_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    title: &str,
) {
    state.session.rename(id, title);
    let (service, id, title) = (services.session.clone(), id.clone(), title.to_string());
    record(spawner, async move {
        service.rename_explorer(&id, &title).await
    });
}

pub fn discard_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
) {
    agent::end(state, id);
    state.session.close(id);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.remove(&id).await });
}

pub fn select(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let now = Timestamp::now();
    state.session.select(id, now);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_seen(&id, now).await });
}

pub fn close(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    agent::end(state, id);
    state.session.close(id);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_opened(&id, None).await });
}

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
    let (service, name) = (services.session.clone(), name.to_string());
    added(spawner, async move {
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
    let (service, repo) = (services.session.clone(), repo.clone());
    added(spawner, async move {
        service.add_worktree(&session, &repo, &spec, None).await
    });
}

pub fn remove_repo(
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    repo: &RepoId,
    force: bool,
) {
    let (service, id, repo) = (services.session.clone(), id.clone(), repo.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.remove_repo(&id, &repo, force).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match result {
                Ok(()) => {
                    if let Some(open) = state.session.get_mut(&id) {
                        open.remove_repo(&repo);
                    }
                    persist_selection(state, services, spawner, &id);
                }
                Err(e) => state.errors.push(e),
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
}

pub fn close_worktree(
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    worktree: &WorktreeId,
    force: bool,
) {
    let (service, id, worktree) = (services.session.clone(), id.clone(), worktree.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.close_worktree(&worktree, force).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match result {
                Ok(closed) => {
                    if let Some(open) = state.session.get_mut(&id) {
                        open.remove_worktree(&closed.id);
                    }
                    persist_selection(state, services, spawner, &id);
                }
                Err(e) => state.errors.push(e),
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
                Err(e) => state.errors.push(e),
            },
        ) as Continuation
    }));
}

/// A provisioning result into the session's row: the repo, the worktree, the notes.
fn added(spawner: &dyn Spawner, work: impl Future<Output = Result<Added, Error>> + Send + 'static) {
    spawner.spawn(Box::pin(async move {
        let result = work.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match result {
                Ok(added) => {
                    let id = added.worktree.session.clone();
                    state.notes.extend(added.notes);
                    if let Some(open) = state.session.get_mut(&id) {
                        open.add_worktree(added.repo, added.worktree);
                    }
                    persist_selection(state, services, spawner, &id);
                }
                Err(e) => state.errors.push(e),
            },
        ) as Continuation
    }));
}

/// Writes the row's selected worktree to its leaf.
fn persist_selection(state: &AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let selected = state
        .session
        .get(id)
        .and_then(|o| o.state.selected_worktree.clone());
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move {
        service.set_selected_worktree(&id, selected.as_ref()).await
    });
}

/// A write whose only result is success or an error for the feed.
fn record(spawner: &dyn Spawner, write: impl Future<Output = Result<(), Error>> + Send + 'static) {
    spawner.spawn(Box::pin(async move {
        let result = write.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = result {
                state.errors.push(e);
            }
        }) as Continuation
    }));
}
