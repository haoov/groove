//! Who asked for a write, and so who hears how it went.

use groove_agent_service::Reply;
use groove_types::Error;

use crate::AppState;

#[derive(Debug)]
pub enum Asker {
    /// The user, from the surface: a failure goes to the feed, and the box is cleared.
    Ui,
    /// The agent, which waits on an answer in words.
    Agent(Reply),
}

impl Asker {
    /// The write landed. The agent hears what it did; the surface shows it.
    pub(crate) fn done(self, said: impl FnOnce() -> String) {
        if let Asker::Agent(reply) = self {
            reply.said(said());
        }
    }

    /// The write failed, wherever it was asked from.
    pub(crate) fn failed(self, state: &mut AppState, e: Error) {
        match self {
            Asker::Ui => state.failed(e),
            Asker::Agent(reply) => reply.failed(e.message),
        }
    }

    /// The write cannot be made at all; only the agent hears it.
    pub(crate) fn refused(self, why: impl Into<String>) {
        if let Asker::Agent(reply) = self {
            reply.failed(why.into());
        }
    }

    /// Whether the words came from a box the surface should now clear.
    pub(crate) fn carries_a_box(&self) -> bool {
        matches!(self, Asker::Ui)
    }

    /// The same result, told either way.
    pub(crate) fn answer(
        self,
        state: &mut AppState,
        done: Result<(), Error>,
        said: impl FnOnce() -> String,
    ) {
        match done {
            Ok(()) => self.done(said),
            Err(e) => self.failed(state, e),
        }
    }
}
