//! What a session says on its MR: a note posted, a thread answered or resolved, a
//! comment, a verdict.

use groove_delivery_service::Said;
use groove_types::{AnnotationId, ReviewVerdict, TimelineKind};

use super::Whose;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// One write the forge answers, on a note or a thread.
#[derive(Debug, Clone, PartialEq)]
pub enum ThreadAct {
    /// The session's note posted on the MR, and resolved here.
    Post {
        id: AnnotationId,
    },
    Reply {
        thread: String,
        body: String,
    },
    Resolve {
        thread: String,
        resolve: bool,
    },
}

impl ThreadAct {
    pub fn id(&self) -> &'static str {
        match self {
            ThreadAct::Post { .. } => "delivery.post_note",
            ThreadAct::Reply { .. } => "delivery.reply_thread",
            ThreadAct::Resolve { .. } => "delivery.resolve_thread",
        }
    }

    /// What it did, in the words the agent reads back.
    fn said(&self) -> &'static str {
        match self {
            ThreadAct::Post { .. } => "posted the note",
            ThreadAct::Reply { .. } => "answered the thread",
            ThreadAct::Resolve { resolve: true, .. } => "resolved the thread",
            ThreadAct::Resolve { resolve: false, .. } => "opened the thread again",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ThreadAct::Post { .. } => "posting the note",
            ThreadAct::Reply { .. } => "replying to the thread",
            ThreadAct::Resolve { .. } => "resolving the thread",
        }
    }

    fn kind(&self) -> TimelineKind {
        match self {
            ThreadAct::Post { .. } => TimelineKind::Note,
            _ => TimelineKind::Review,
        }
    }
}

/// What the commit box says on the MR itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Say {
    Comment,
    Review(ReviewVerdict),
}

impl Say {
    pub fn id(&self) -> &'static str {
        match self {
            Say::Comment => "delivery.comment",
            Say::Review(_) => "delivery.review",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Say::Comment => "commenting on the merge request",
            Say::Review(_) => "reviewing the merge request",
        }
    }

    fn said(&self) -> &'static str {
        match self {
            Say::Comment => "commented on the merge request",
            Say::Review(_) => "left the review",
        }
    }
}

/// A write on the selected worktree's threads, from the surface.
pub(super) fn here(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: ThreadAct,
) {
    if let Some(whose) = Whose::selected(state) {
        thread(state, services, spawner, act, whose, Asker::Ui);
    }
}

/// One write the forge answers, then the session's notes and its MR read again.
pub(crate) fn thread(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: ThreadAct,
    whose: Whose,
    asker: Asker,
) {
    let Some(call) = call_of(state, &whose, act.clone()) else {
        return asker.refused("this session left no note by that id");
    };
    let (service, job) = (services.delivery.clone(), state.begin(act.label()));
    let told = act;
    spawner.spawn(Box::pin(async move {
        let (repo, at) = (&whose.repo, &whose.worktree.id);
        let done = match call {
            Call::Post(note) => service.post_note(repo, at, &note).await,
            Call::Reply { thread, body } => service.reply_thread(repo, at, &thread, &body).await,
            Call::Resolve { thread, resolve } => {
                service.resolve_thread(repo, &thread, resolve).await
            }
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    let (kind, at) = (told.kind(), &whose.worktree.id);
                    crate::tools::logged(services, spawner, &whose.session, kind, told.said(), at);
                    landed(state, services, spawner, &whose);
                }
                asker.answer(state, done, || told.said().to_string());
            },
        ) as Continuation
    }));
}

/// The act with the note it posts in hand.
enum Call {
    Post(groove_types::Annotation),
    Reply { thread: String, body: String },
    Resolve { thread: String, resolve: bool },
}

fn call_of(state: &AppState, whose: &Whose, act: ThreadAct) -> Option<Call> {
    Some(match act {
        ThreadAct::Post { id } => Call::Post(state.delivery.note(&whose.session, &id)?.clone()),
        ThreadAct::Reply { thread, body } => Call::Reply { thread, body },
        ThreadAct::Resolve { thread, resolve } => Call::Resolve { thread, resolve },
    })
}

/// The commit box's words on the selected worktree's MR.
pub(super) fn say_here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, say: Say) {
    let Some(whose) = Whose::selected(state) else {
        return;
    };
    let body = state.workspace.message.text().trim().to_string();
    say_as(state, services, spawner, say, body, whose, Asker::Ui);
}

/// A comment, or a verdict carrying every note of the session the forge has not seen.
pub(crate) fn say_as(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    say: Say,
    body: String,
    whose: Whose,
    asker: Asker,
) {
    if say == Say::Comment && body.is_empty() {
        return asker.refused("a comment needs a body");
    }
    let notes = state.delivery.unposted(&whose.session, &whose.repo.id);
    let (service, job) = (services.delivery.clone(), state.begin(say.label()));
    let clears = asker.carries_a_box();
    spawner.spawn(Box::pin(async move {
        let (repo, at) = (&whose.repo, &whose.worktree.id);
        let done = match say {
            Say::Comment => service.comment(repo, at, &body).await,
            Say::Review(verdict) => {
                let said = Said {
                    verdict,
                    body: &body,
                    notes: &notes,
                };
                service.review(repo, at, said).await
            }
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    if clears {
                        state.workspace.message = groove_workspace_service::Buffer::default();
                    }
                    let (kind, at) = (TimelineKind::Review, &whose.worktree.id);
                    crate::tools::logged(services, spawner, &whose.session, kind, say.label(), at);
                    landed(state, services, spawner, &whose);
                }
                asker.answer(state, done, || say.said().to_string());
            },
        ) as Continuation
    }));
}

/// The session's notes read again, and its MR with them: a thread only the forge holds.
fn landed(state: &mut AppState, services: &Services, spawner: &dyn Spawner, whose: &Whose) {
    super::notes::list(state, services, spawner, &whose.session);
    super::poll::refresh(state, services, spawner, &whose.worktree.id);
}
