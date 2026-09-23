//! The writes the forge answers: a note posted, a thread answered, a verdict given.

use groove_types::{Annotation, Result};
use groove_workspace_service::{Remote, Service};

use super::notes::{Act, Whose};
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// One write against a worktree's forge, then the notes read again.
pub(super) fn note(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: Act,
    whose: Whose,
    asker: Asker,
) {
    let (repo, id) = (whose.repo.clone(), whose.worktree.clone());
    let posted = match &act {
        Act::Post { id } => match held(state, id) {
            Some(note) => Some(note),
            None => return asker.refused("this session left no note by that id"),
        },
        _ => None,
    };
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return asker.failed(state, e),
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
                if done.is_ok() {
                    let kind = kind_of(&told);
                    let at = &whose.worktree;
                    let subject = subject(&told);
                    crate::tools::logged(services, spawner, &whose.session, kind, &subject, at);
                    landed(state, services, spawner);
                }
                asker.answer(state, done, || told.said());
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

    /// What it did, in the words the agent reads back.
    fn said(&self) -> &'static str {
        match self {
            Say::Comment => "commented on the merge request",
            Say::Review(_) => "left the review",
        }
    }
}

/// The words of the commit box on the merge request: a comment, or a verdict with
/// every note this session has not posted.
pub(super) fn here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, said: Say) {
    let Some(whose) = super::notes::selected(state) else {
        return;
    };
    let body = state.workspace.message.text().trim().to_string();
    if said == Say::Comment && body.is_empty() {
        return;
    }
    say(state, services, spawner, said, body, whose, Asker::Ui);
}

/// The same, on the worktree and with the words the caller names.
pub(crate) fn say(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    say: Say,
    body: String,
    whose: Whose,
    asker: Asker,
) {
    let (repo, id) = (whose.repo.clone(), whose.worktree.clone());
    if say == Say::Comment && body.is_empty() {
        return asker.refused("a comment needs a body");
    }
    let notes = pending(state, &repo.id);
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return state.failed(e),
    };
    let service = services.workspace.clone();
    let job = state.begin(say.label());
    let told = say.clone();
    let clears = asker.carries_a_box();
    spawner.spawn(Box::pin(async move {
        let done = said(&service, &remote, &repo, &id, &say, &body, &notes).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    if clears {
                        state.workspace.message = groove_workspace_service::Buffer::default();
                    }
                    let kind = groove_types::TimelineKind::Review;
                    let at = &whose.worktree;
                    let subject = told.label();
                    crate::tools::logged(services, spawner, &whose.session, kind, subject, at);
                    landed(state, services, spawner);
                }
                asker.answer(state, done, || told.said().to_string());
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
