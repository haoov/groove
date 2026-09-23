//! The writes the forge answers: a note posted, a thread answered, a verdict given.

use groove_types::{Annotation, Result};
use groove_workspace_service::{Remote, Service};

use super::notes::Act;
use crate::{AppState, Continuation, Services, Spawner};

/// One write against the selected worktree's forge, then the notes read again.
pub(super) fn note(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: Act) {
    let Some(id) = super::selected(state) else {
        return;
    };
    let Some((repo, _)) = super::pair(state, &id) else {
        return;
    };
    let posted = match &act {
        Act::Post { id } => match held(state, id) {
            Some(note) => Some(note),
            None => return,
        },
        _ => None,
    };
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return state.failed(e),
    };
    let service = services.workspace.clone();
    let job = state.begin(act.label());
    let act = act.clone();
    let told = act.clone();
    spawner.spawn(Box::pin(async move {
        let done = made(&service, &remote, &repo, &id, act, posted).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Err(e) => state.failed(e),
                    Ok(()) => {
                        let kind = kind_of(&told);
                        crate::timeline::log(state, services, spawner, kind, subject(&told));
                        landed(state, services, spawner);
                    }
                }
            },
        ) as Continuation
    }));
}

async fn made(
    service: &Service,
    remote: &Remote,
    repo: &groove_types::Repo,
    worktree: &groove_types::WorktreeId,
    act: Act,
    posted: Option<Annotation>,
) -> Result<()> {
    match act {
        Act::Post { .. } => match posted {
            Some(note) => service.post_note(remote, repo, worktree, &note).await,
            None => Ok(()),
        },
        Act::Reply { thread, body } => {
            service
                .reply_thread(remote, repo, worktree, &thread, &body)
                .await
        }
        Act::Thread { thread, resolve } => service.resolve_thread(remote, &thread, resolve).await,
        _ => Ok(()),
    }
}

/// The line one of these writes leaves.
fn kind_of(act: &Act) -> groove_types::TimelineKind {
    match act {
        Act::Post { .. } => groove_types::TimelineKind::Note,
        _ => groove_types::TimelineKind::Review,
    }
}

/// What the line says it was about.
fn subject(act: &Act) -> String {
    match act {
        Act::Post { .. } => "posted".to_string(),
        Act::Reply { .. } => "replied".to_string(),
        Act::Thread { resolve, .. } => match resolve {
            true => "thread resolved".to_string(),
            false => "thread opened again".to_string(),
        },
        _ => String::new(),
    }
}

/// The note the id names, as this session's rows hold it.
fn held(state: &AppState, id: &groove_types::AnnotationId) -> Option<Annotation> {
    state
        .workspace
        .own
        .iter()
        .find(|note| &note.id == id)
        .cloned()
}

/// The notes read again, and the MR with them: a thread only the forge holds.
fn landed(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    super::notes::list(state, services, spawner);
    super::mr::refresh(state, services, spawner);
}

/// What a review or a comment leaves on the merge request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Say {
    Comment,
    Review(groove_types::ReviewVerdict),
}

impl Say {
    pub fn id(&self) -> &'static str {
        match self {
            Say::Comment => "workspace.comment",
            Say::Review(_) => "workspace.review",
        }
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            Say::Comment => "commenting on the merge request",
            Say::Review(_) => "reviewing the merge request",
        }
    }
}

/// The words of the commit box on the merge request: a comment, or a verdict with
/// every note this session has not posted.
pub(super) fn say(state: &mut AppState, services: &Services, spawner: &dyn Spawner, say: Say) {
    let Some(id) = super::selected(state) else {
        return;
    };
    let Some((repo, _)) = super::pair(state, &id) else {
        return;
    };
    let body = state.workspace.message.text().trim().to_string();
    if say == Say::Comment && body.is_empty() {
        return;
    }
    let notes = pending(state, &repo.id);
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return state.failed(e),
    };
    let service = services.workspace.clone();
    let job = state.begin(say.label());
    let told = say.clone();
    spawner.spawn(Box::pin(async move {
        let done = said(&service, &remote, &repo, &id, &say, &body, &notes).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Err(e) => state.failed(e),
                    Ok(()) => {
                        state.workspace.message = groove_workspace_service::Buffer::default();
                        let kind = groove_types::TimelineKind::Review;
                        crate::timeline::log(state, services, spawner, kind, told.label());
                        landed(state, services, spawner);
                    }
                }
            },
        ) as Continuation
    }));
}

async fn said(
    service: &Service,
    remote: &Remote,
    repo: &groove_types::Repo,
    worktree: &groove_types::WorktreeId,
    say: &Say,
    body: &str,
    notes: &[Annotation],
) -> Result<()> {
    match say {
        Say::Comment => service.comment(remote, repo, worktree, body).await,
        Say::Review(verdict) => {
            let said = groove_workspace_service::Said {
                verdict: *verdict,
                body,
                notes,
            };
            service.review(remote, repo, worktree, said).await
        }
    }
}

/// The notes of this repo the forge has not been told about.
fn pending(state: &AppState, repo: &groove_types::RepoId) -> Vec<Annotation> {
    state
        .workspace
        .own
        .iter()
        .filter(|note| &note.repo == repo)
        .filter(|note| note.status == groove_types::AnnotationStatus::Open)
        .cloned()
        .collect()
}
