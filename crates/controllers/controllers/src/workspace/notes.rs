//! The session's notes: read with the MR's threads, written on their own.

use groove_types::{Anchor, Annotation, AnnotationId, RepoId, Result, SessionId, Timestamp};
use groove_workspace_service::NewNote;

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// One write on a note, named as the ask that sends it.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
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
    /// This session's note posted on the merge request, and resolved here.
    Post {
        id: AnnotationId,
    },
    Reply {
        thread: String,
        body: String,
    },
    Thread {
        thread: String,
        resolve: bool,
    },
}

impl Act {
    /// Whether the forge answers it, rather than this session's own rows.
    fn forged(&self) -> bool {
        matches!(
            self,
            Act::Post { .. } | Act::Reply { .. } | Act::Thread { .. }
        )
    }

    pub fn id(&self) -> &'static str {
        match self {
            Act::Create { .. } => "workspace.create_note",
            Act::Update { .. } => "workspace.update_note",
            Act::Resolve { .. } => "workspace.resolve_note",
            Act::Reopen { .. } => "workspace.reopen_note",
            Act::Delete { .. } => "workspace.delete_note",
            Act::Post { .. } => "workspace.post_note",
            Act::Reply { .. } => "workspace.reply_thread",
            Act::Thread { .. } => "workspace.resolve_thread",
        }
    }

    /// What it did, in the words the agent reads back.
    pub(crate) fn said(&self) -> String {
        match self {
            Act::Create { anchor, .. } => {
                format!("noted {} {}", anchor.path, anchor.start_line + 1)
            }
            Act::Update { .. } => "wrote the note again".into(),
            Act::Resolve { .. } => "resolved the note".into(),
            Act::Reopen { .. } => "opened the note again".into(),
            Act::Delete { .. } => "deleted the note".into(),
            Act::Post { .. } => "posted the note".into(),
            Act::Reply { .. } => "answered the thread".into(),
            Act::Thread { resolve, .. } => match resolve {
                true => "resolved the thread".into(),
                false => "opened the thread again".into(),
            },
        }
    }

    pub(super) fn label(&self) -> &'static str {
        match self {
            Act::Create { .. } => "leaving a note",
            Act::Update { .. } => "writing the note again",
            Act::Resolve { .. } => "resolving the note",
            Act::Reopen { .. } => "opening the note again",
            Act::Delete { .. } => "deleting the note",
            Act::Post { .. } => "posting the note",
            Act::Reply { .. } => "replying to the thread",
            Act::Thread { .. } => "resolving the thread",
        }
    }
}

/// The notes of the selected session, beside the threads the MR already answered.
pub(crate) fn list(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let Some(session) = state.session.selected.clone() else {
        state.workspace.own.clear();
        state.workspace.noted = None;
        return state.workspace.remerge();
    };
    state.workspace.noted = Some(session.clone());
    let service = services.workspace.clone();
    let job = state.begin("reading the notes");
    spawner.spawn(Box::pin(async move {
        let read = service.notes(&session).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            held(state, &session, read);
        }) as Continuation
    }));
}

/// Whose note it is: the session that leaves it, and the worktree it stands on.
#[derive(Debug, Clone)]
pub(crate) struct Whose {
    pub session: SessionId,
    pub repo: groove_types::Repo,
    pub worktree: groove_types::WorktreeId,
}

/// The selected session and worktree, which the surface's own notes belong to.
pub(super) fn here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: Act) {
    let Some(whose) = selected(state) else {
        return;
    };
    write(state, services, spawner, act, whose, Asker::Ui);
}

/// One write, then the list read again.
pub(crate) fn write(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: Act,
    whose: Whose,
    asker: Asker,
) {
    if let Act::Create { anchor, .. } = &act
        && noted(state, &anchor.path, (anchor.start_line, anchor.end_line))
    {
        return asker.refused(ALREADY_NOTED);
    }
    if act.forged() {
        return super::forge::note(state, services, spawner, act, whose, asker);
    }
    let service = services.workspace.clone();
    let job = state.begin(act.label());
    let left = left_by(&act);
    let said = act.said();
    let (session, repo) = (whose.session.clone(), whose.repo.id.clone());
    spawner.spawn(Box::pin(async move {
        let done = apply(&service, act, session, repo, Timestamp::now()).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    if let Some(subject) = left.as_ref() {
                        let kind = groove_types::TimelineKind::Note;
                        let at = &whose.worktree;
                        crate::tools::logged(services, spawner, &whose.session, kind, subject, at);
                    }
                    list(state, services, spawner);
                }
                asker.answer(state, done, || said);
            },
        ) as Continuation
    }));
}

pub(crate) const ALREADY_NOTED: &str = "a note of this session already stands on one of those lines: write that one again, \
     or resolve it first";

/// Whether an open note of this session already covers any of those lines.
fn noted(state: &AppState, path: &str, lines: (u32, u32)) -> bool {
    state
        .workspace
        .notes
        .iter()
        .filter(|note| note.is_local() && !note.resolved)
        .any(|note| note.over(path, lines))
}

async fn apply(
    service: &groove_workspace_service::Service,
    act: Act,
    session: SessionId,
    repo: RepoId,
    now: Timestamp,
) -> Result<()> {
    match act {
        Act::Create {
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
            service.create_note(new, now).await.map(drop)
        }
        Act::Update { id, content } => service.update_note(&id, &content).await.map(drop),
        Act::Resolve { id } => service.resolve_note(&id).await.map(drop),
        Act::Reopen { id } => service.reopen_note(&id).await.map(drop),
        Act::Delete { id } => service.delete_note(&id).await,
        Act::Post { .. } | Act::Reply { .. } | Act::Thread { .. } => Ok(()),
    }
}

/// What a write leaves on the log, or nothing for one that only moves a note about.
fn left_by(act: &Act) -> Option<String> {
    match act {
        Act::Create { anchor, .. } => Some(format!("{} {}", anchor.path, anchor.start_line + 1)),
        _ => None,
    }
}

/// The notes the read answered, dropped when the selection moved on.
fn held(state: &mut AppState, session: &SessionId, read: Result<Vec<Annotation>>) {
    if state.session.selected.as_ref() != Some(session) {
        return;
    }
    match read {
        Err(e) => state.failed(e),
        Ok(own) => {
            state.workspace.own = own;
            state.workspace.remerge();
        }
    }
}

/// The session a note belongs to, and the worktree its file is in.
pub(crate) fn selected(state: &AppState) -> Option<Whose> {
    let open = state.session.selected()?;
    let worktree = open.selected_worktree()?;
    let repo = open.repos.iter().find(|one| one.id == worktree.repo)?;
    Some(Whose {
        session: open.session.id.clone(),
        repo: repo.clone(),
        worktree: worktree.id.clone(),
    })
}
