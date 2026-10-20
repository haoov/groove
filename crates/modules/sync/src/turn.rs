//! One task's place in a line of tasks that run in the order asked.

use tokio::sync::oneshot::{Receiver, Sender, channel};

/// What the task before waits on; ends when its `Turn` ends or is dropped.
pub type Ended = Receiver<()>;

/// A task's place in its line: the task before it to wait for, and its own end.
#[derive(Debug)]
pub struct Turn {
    before: Option<Ended>,
    done: Sender<()>,
}

impl Turn {
    /// A new last place: `line` keeps this one's end and hands back the end it kept before.
    pub fn behind(line: impl FnOnce(Ended) -> Option<Ended>) -> Self {
        let (done, last) = channel();
        let before = line(last);
        Self { before, done }
    }

    /// Waits until the task before this one has ended.
    pub async fn wait(&mut self) {
        if let Some(before) = self.before.take() {
            let _ = before.await;
        }
    }

    /// This task ended; the next one in line may run. Dropping the turn does the same.
    pub fn end(self) {
        let _ = self.done.send(());
    }
}
