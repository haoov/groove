//! The writes waiting on a human, and what each one holds until it is decided.

#[cfg(test)]
mod tests;

use groove_types::{Approval, ApprovalId, Ask, Origin, SessionId, Timestamp};

/// The queue of writes nobody has decided on. `T` is what the caller holds until then.
#[derive(Debug)]
pub struct Queue<T> {
    waiting: Vec<(Approval, T)>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self {
            waiting: Vec::new(),
        }
    }
}

impl<T> Queue<T> {
    /// One write queued, and the approval it stands as.
    pub fn queue(&mut self, new: New, held: T) -> Approval {
        let approval = Approval {
            id: ApprovalId::new(uuid::Uuid::new_v4().simple().to_string()),
            session: Some(new.session),
            op: new.op,
            payload: new.payload,
            origin: new.origin,
            created_at: new.at,
            claimed_at: None,
        };
        self.waiting.push((approval.clone(), held));
        approval
    }

    /// The write one id names, taken off the queue.
    pub fn resolve(&mut self, id: &ApprovalId) -> Option<(Approval, T)> {
        let at = self.waiting.iter().position(|(one, _)| &one.id == id)?;
        Some(self.waiting.remove(at))
    }

    /// Every write of that session, taken off the queue.
    pub fn forget(&mut self, session: &SessionId) -> Vec<(Approval, T)> {
        let (theirs, rest) = std::mem::take(&mut self.waiting)
            .into_iter()
            .partition(|(one, _)| one.session.as_ref() == Some(session));
        self.waiting = rest;
        theirs
    }

    /// What that session waits on, each one as `ask` says it.
    pub fn asks(&self, session: &SessionId, ask: impl Fn(&Approval) -> Ask) -> Vec<Ask> {
        self.waiting
            .iter()
            .filter(|(one, _)| one.session.as_ref() == Some(session))
            .map(|(one, _)| ask(one))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.waiting.len()
    }

    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }
}

/// What a write is, before it stands as an approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct New {
    pub session: SessionId,
    pub op: String,
    pub payload: serde_json::Value,
    pub origin: Origin,
    pub at: Timestamp,
}
