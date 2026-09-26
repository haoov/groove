//! A session's own notes: read, and written on the database alone.

use groove_delivery_service::NewNote;
use groove_types::{Anchor, AnnotationId, SessionId, TimelineKind, Timestamp};

use super::Whose;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// One write on a note of the session.
#[derive(Debug, Clone, PartialEq)]
pub enum NoteAct {
    Create {
        anchor: Anchor,
        content: String,
        author: String,
    },
    Update {
        id: AnnotationId,
        content: String,
    },
    Resolve {
        id: AnnotationId,
    },
    Reopen {
        id: AnnotationId,
    },
    Delete {
        id: AnnotationId,
    },
}

impl NoteAct {
    pub fn id(&self) -> &'static str {
        match self {
            NoteAct::Create { .. } => "delivery.create_note",
            NoteAct::Update { .. } => "delivery.update_note",
            NoteAct::Resolve { .. } => "delivery.resolve_note",
            NoteAct::Reopen { .. } => "delivery.reopen_note",
            NoteAct::Delete { .. } => "delivery.delete_note",
        }
    }

    /// What it did, in the words the agent reads back.
    fn said(&self) -> String {
        match self {
            NoteAct::Create { anchor, .. } => {
                format!("noted {} {}", anchor.path, anchor.start_line + 1)
            }
            NoteAct::Update { .. } => "wrote the note again".into(),
            NoteAct::Resolve { .. } => "resolved the note".into(),
            NoteAct::Reopen { .. } => "opened the note again".into(),
            NoteAct::Delete { .. } => "deleted the note".into(),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            NoteAct::Create { .. } => "leaving a note",
            NoteAct::Update { .. } => "writing the note again",
            NoteAct::Resolve { .. } => "resolving the note",
            NoteAct::Reopen { .. } => "opening the note again",
            NoteAct::Delete { .. } => "deleting the note",
        }
    }
}

pub(crate) const ALREADY_NOTED: &str = "a note of this session already stands on one of those lines: write that one again, \
     or resolve it first";

/// The selected session's notes.
pub(super) fn list_selected(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    if let Some(session) = state.session.selected.clone() {
        list(state, services, spawner, &session);
    }
}

/// One session's notes, read into the service's state.
pub(crate) fn list(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
) {
    state.delivery.reading(session);
    let (service, session) = (services.delivery.clone(), session.clone());
    let job = state.begin("reading the notes");
    spawner.spawn(Box::pin(async move {
        let read = service.notes(&session).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match read {
                Ok(notes) => state.delivery.noted(session, notes),
                Err(e) => state.failed(e),
            }
            show(state);
        }) as Continuation
    }));
}

/// The notes the surface shows: the selected session's, beside its worktree's threads.
pub(crate) fn show(state: &mut AppState) {
    let session = state.session.selected.clone();
    let worktree = state.session.selected_worktree().map(|one| one.id.clone());
    state.delivery.show(session.as_ref(), worktree.as_ref());
}

/// A write on the selected session's notes, from the surface.
pub(super) fn here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: NoteAct) {
    if let Some(whose) = Whose::selected(state) {
        write(state, services, spawner, act, whose, Asker::Ui);
    }
}

/// One write on a note of the session it belongs to, then its notes read again.
pub(crate) fn write(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: NoteAct,
    whose: Whose,
    asker: Asker,
) {
    if let NoteAct::Create { anchor, .. } = &act {
        let lines = (anchor.start_line, anchor.end_line);
        if state.delivery.covered(&whose.session, &anchor.path, lines) {
            return asker.refused(ALREADY_NOTED);
        }
    }
    let (service, job) = (services.delivery.clone(), state.begin(act.label()));
    let (said, left) = (act.said(), left_by(&act));
    let (session, repo) = (whose.session.clone(), whose.repo.id.clone());
    spawner.spawn(Box::pin(async move {
        let done = apply(&service, act, session, repo).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    if let Some(subject) = left {
                        let (kind, at) = (TimelineKind::Note, &whose.worktree.id);
                        crate::tools::logged(services, spawner, &whose.session, kind, &subject, at);
                    }
                    list(state, services, spawner, &whose.session);
                }
                asker.answer(state, done, || said);
            },
        ) as Continuation
    }));
}

async fn apply(
    service: &groove_delivery_service::Service,
    act: NoteAct,
    session: SessionId,
    repo: groove_types::RepoId,
) -> groove_types::Result<()> {
    match act {
        NoteAct::Create {
            anchor,
            content,
            author,
        } => {
            let new = NewNote {
                session,
                repo,
                file_path: anchor.path,
                start_line: anchor.start_line,
                end_line: anchor.end_line,
                content,
                author,
            };
            service.create_note(new, Timestamp::now()).await.map(drop)
        }
        NoteAct::Update { id, content } => service.update_note(&id, &content).await.map(drop),
        NoteAct::Resolve { id } => service.resolve_note(&id).await.map(drop),
        NoteAct::Reopen { id } => service.reopen_note(&id).await.map(drop),
        NoteAct::Delete { id } => service.delete_note(&id).await,
    }
}

/// The line a new note leaves on the log; the other writes leave none.
fn left_by(act: &NoteAct) -> Option<String> {
    match act {
        NoteAct::Create { anchor, .. } => {
            Some(format!("{} {}", anchor.path, anchor.start_line + 1))
        }
        _ => None,
    }
}
