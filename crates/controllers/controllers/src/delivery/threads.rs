//! What a session says on its MR: a thread opened, answered or resolved, a verdict.

use groove_delivery_service::Said;
use groove_types::{Anchor, ReviewVerdict, TimelineKind};

use super::Whose;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// One write the forge answers, on a thread.
#[derive(Debug, Clone, PartialEq)]
pub enum ThreadAct {
    /// A new thread on lines of the diff.
    Comment {
        anchor: Anchor,
        body: String,
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
            ThreadAct::Comment { .. } => "delivery.comment_lines",
            ThreadAct::Reply { .. } => "delivery.reply_thread",
            ThreadAct::Resolve { .. } => "delivery.resolve_thread",
        }
    }

    /// What it did, in the words the agent reads back.
    fn said(&self) -> &'static str {
        match self {
            ThreadAct::Comment { .. } => "opened a thread",
            ThreadAct::Reply { .. } => "answered the thread",
            ThreadAct::Resolve { resolve: true, .. } => "resolved the thread",
            ThreadAct::Resolve { resolve: false, .. } => "opened the thread again",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            ThreadAct::Comment { .. } => "opening a thread",
            ThreadAct::Reply { .. } => "replying to the thread",
            ThreadAct::Resolve { .. } => "resolving the thread",
        }
    }

    fn kind(&self) -> TimelineKind {
        match self {
            ThreadAct::Comment { .. } => TimelineKind::Note,
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

/// One write the forge answers, then the MR read again.
pub(crate) fn thread(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    act: ThreadAct,
    whose: Whose,
    asker: Asker,
) {
    let (service, job) = (services.delivery.clone(), state.begin(act.label()));
    let told = act.clone();
    spawner.spawn(Box::pin(async move {
        let (repo, at) = (&whose.repo, &whose.worktree.id);
        let done = match act {
            ThreadAct::Comment { anchor, body } => {
                service.comment_lines(repo, at, &anchor, &body).await
            }
            ThreadAct::Reply { thread, body } => {
                service.reply_thread(repo, at, &thread, &body).await
            }
            ThreadAct::Resolve { thread, resolve } => {
                service.resolve_thread(repo, &thread, resolve).await
            }
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    let (kind, at) = (told.kind(), &whose.worktree.id);
                    crate::timeline::log(services, spawner, &whose.session, kind, told.said(), at);
                    landed(state, services, spawner, &whose);
                }
                asker.answer(state, done, || told.said().to_string());
            },
        ) as Continuation
    }));
}

/// A comment or a verdict on the selected worktree's MR.
pub(super) fn say_here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, say: Say) {
    let Some(whose) = Whose::selected(state) else {
        return;
    };
    say_as(
        state,
        services,
        spawner,
        say,
        String::new(),
        whose,
        Asker::Ui,
    );
}

/// A comment, or a verdict.
pub(crate) fn say_as(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    say: Say,
    body: String,
    whose: Whose,
    asker: Asker,
) {
    let verdict = match &say {
        Say::Comment => None,
        Say::Review(verdict) => Some(*verdict),
    };
    if let Err(e) = Said::empty(verdict, &body) {
        return asker.failed(state, e);
    }
    let (service, job) = (services.delivery.clone(), state.begin(say.label()));
    spawner.spawn(Box::pin(async move {
        let (repo, at) = (&whose.repo, &whose.worktree.id);
        let done = match say {
            Say::Comment => service.comment(repo, at, &body).await,
            Say::Review(verdict) => {
                let said = Said {
                    verdict,
                    body: &body,
                };
                service.review(repo, at, said).await
            }
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if done.is_ok() {
                    spoke(state, services, spawner, (&say, &whose));
                }
                asker.answer(state, done, || say.said().to_string());
            },
        ) as Continuation
    }));
}

/// The timeline told, the MR read again.
fn spoke(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (say, whose): (&Say, &Whose),
) {
    let (kind, at) = (TimelineKind::Review, &whose.worktree.id);
    crate::timeline::log(services, spawner, &whose.session, kind, say.label(), at);
    landed(state, services, spawner, whose);
}

/// The MR read again, its threads with it.
fn landed(state: &mut AppState, services: &Services, spawner: &dyn Spawner, whose: &Whose) {
    super::poll::refresh(state, services, spawner, &whose.worktree.id);
}
