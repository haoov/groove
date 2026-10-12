//! The launch-file writes of each session, run in the order asked, and the start they hold back.

use groove_types::SessionId;
use tokio::sync::oneshot::{Receiver, Sender, channel};

/// A start asked while a session's launch files were being written: its size and first words.
pub type Held = ((u16, u16), Option<String>);

/// One write's place in its session's line: the write before it to wait for, and its own end.
pub struct Turn {
    before: Option<Receiver<()>>,
    done: Sender<()>,
}

impl Turn {
    /// Waits until the write asked before this one has ended.
    pub async fn wait(&mut self) {
        if let Some(before) = self.before.take() {
            let _ = before.await;
        }
    }

    /// This write ended; the next one in line may run.
    pub fn end(self) {
        let _ = self.done.send(());
    }
}

/// The writes out for one session, and the end the next one waits for.
#[derive(Debug)]
pub(crate) struct Line {
    session: SessionId,
    out: u32,
    last: Option<Receiver<()>>,
    held: Option<Held>,
}

impl crate::State {
    /// One more write of `session`'s launch files, behind the ones out.
    pub fn writing(&mut self, session: &SessionId) -> Turn {
        let (done, last) = channel();
        let at = match self.writes.iter().position(|one| &one.session == session) {
            Some(at) => at,
            None => {
                self.writes.push(Line {
                    session: session.clone(),
                    out: 0,
                    last: None,
                    held: None,
                });
                self.writes.len() - 1
            }
        };
        let line = &mut self.writes[at];
        line.out += 1;
        let before = line.last.replace(last);
        Turn { before, done }
    }

    /// One write landed; the start it held back once none is out.
    pub fn written(&mut self, session: &SessionId) -> Option<Held> {
        let at = self.writes.iter().position(|one| &one.session == session)?;
        self.writes[at].out -= 1;
        if self.writes[at].out > 0 {
            return None;
        }
        self.writes.remove(at).held
    }

    /// The start kept for later while a write is out, or given back when none is.
    pub fn hold(&mut self, session: &SessionId, start: Held) -> Option<Held> {
        let Some(line) = self.writes.iter_mut().find(|one| &one.session == session) else {
            return Some(start);
        };
        match &mut line.held {
            Some(kept) if start.1.is_none() => kept.0 = start.0,
            held => *held = Some(start),
        }
        None
    }
}
