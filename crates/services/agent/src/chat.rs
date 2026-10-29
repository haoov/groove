//! Each agent's conversation, heard from the transcript it writes.

use std::path::{Path, PathBuf};

use groove_transcript::{Heeded, Stop};
use groove_types::SessionId;
use groove_types::chat::{Chat, Heard};

use crate::State;

/// One session's transcript, its follower and what it said.
#[derive(Debug)]
pub(crate) struct Followed {
    path: PathBuf,
    _stop: Stop,
    chat: Chat,
}

impl State {
    /// The transcript the session's conversation is read from.
    pub fn transcript(&self, session: &SessionId) -> Option<&Path> {
        self.followed(session).map(|one| one.path.as_path())
    }

    pub fn chat(&self, session: &SessionId) -> Option<&Chat> {
        self.followed(session).map(|one| &one.chat)
    }

    /// The conversation read again from `path`, its last follower ended; what the new one listens to.
    pub fn follow(&mut self, session: &SessionId, path: PathBuf) -> Option<Heeded> {
        self.agent(session)?;
        self.unfollow(session);
        let (stop, heeded) = groove_transcript::stop();
        let followed = Followed {
            path,
            _stop: stop,
            chat: Chat::default(),
        };
        self.chats.push((session.clone(), followed));
        Some(heeded)
    }

    pub(crate) fn unfollow(&mut self, session: &SessionId) {
        self.chats.retain(|(id, _)| id != session);
    }

    /// What one read of `path` heard; a transcript no longer followed is not heard.
    pub(crate) fn heard(&mut self, session: &SessionId, path: &Path, heard: Vec<Heard>) {
        let followed = self.chats.iter_mut().find(|(id, _)| id == session);
        let Some((_, followed)) = followed.filter(|(_, one)| one.path == path) else {
            return;
        };
        heard.into_iter().for_each(|one| followed.chat.hear(one));
    }

    /// Words sent from Groove's own prompt, waiting until the transcript says them.
    pub(crate) fn echoed(&mut self, session: &SessionId, text: String) {
        let followed = self.chats.iter_mut().find(|(id, _)| id == session);
        if let Some((_, followed)) = followed {
            followed.chat.hear(Heard::Sent(text));
        }
    }

    fn followed(&self, session: &SessionId) -> Option<&Followed> {
        let found = self.chats.iter().find(|(id, _)| id == session);
        found.map(|(_, one)| one)
    }
}

/// Reads `path` from its start until `heeded` stops, each batch through `send`.
pub async fn follower(path: PathBuf, heeded: Heeded, send: impl Fn(Vec<Heard>) + Send) {
    groove_transcript::follow(path, heeded, send).await
}
