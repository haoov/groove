//! Which routines an event starts, and where: once a session, not the selected one, auto-approve on.

use groove_types::{Action, Routine, RoutineKind, RoutinesConfig, SessionId};

use super::{Fired, Run, Runs};

/// How a run's words reach its agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deliver {
    /// Typed into the waiting agent.
    Type,
    /// Asked of a new agent on the session's conversation.
    Ask,
    Nothing,
}

pub fn deliver(kind: RoutineKind, alive: bool) -> Deliver {
    match (kind, alive) {
        (RoutineKind::Bound, true) => Deliver::Type,
        (RoutineKind::Bound, false) => Deliver::Ask,
        (RoutineKind::Action, _) => Deliver::Nothing,
    }
}

/// One open session as the rules weigh it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub id: SessionId,
    pub auto_approve: bool,
}

/// The rail as the rules weigh it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sessions {
    pub open: Vec<Place>,
    pub selected: Option<SessionId>,
}

impl Runs {
    /// Every switched-on routine that answers to the event, queued where it runs; the actions to do.
    pub fn fire(
        &mut self,
        fired: &Fired,
        routines: &[Routine],
        held: &RoutinesConfig,
        sessions: &Sessions,
    ) -> Vec<Action> {
        let quiet = |one: &Routine| {
            held.quiet
                .get(&one.id)
                .is_some_and(|q| q.contains(&fired.trigger))
        };
        let answering = routines
            .iter()
            .filter(|one| held.on.contains(&one.id) && one.on.contains(&fired.trigger))
            .filter(|one| !quiet(one));
        let mut actions = Vec::new();
        for routine in answering {
            if let Some(action) = routine.action {
                actions.push(action);
                continue;
            }
            let Some(session) = target(routine, fired.session.as_ref(), sessions) else {
                continue;
            };
            let taken =
                self.queued_since_seen(&routine.id, &session) || self.holds(&routine.id, &session);
            if taken || !acts(sessions, &session) {
                continue;
            }
            self.queue(Run::new(
                &routine.id,
                session,
                Some(fired.trigger),
                &fired.about,
            ));
        }
        actions
    }
}

/// Whether an event may act on the session: not the selected one, only where auto-approve is on.
fn acts(sessions: &Sessions, session: &SessionId) -> bool {
    let selected = sessions.selected.as_ref() == Some(session);
    let place = sessions.open.iter().find(|one| &one.id == session);
    place.is_some_and(|one| one.auto_approve) && !selected
}

/// A bound routine runs on the open session the event is about.
pub fn target(
    routine: &Routine,
    about: Option<&SessionId>,
    sessions: &Sessions,
) -> Option<SessionId> {
    if routine.kind != RoutineKind::Bound {
        return None;
    }
    let mut open = sessions.open.iter();
    open.find(|one| Some(&one.id) == about)
        .map(|one| one.id.clone())
}

/// What pressing `routine`'s button does, whatever the rules: its action; or why it does nothing.
pub fn pressed(routine: &Routine) -> Result<Action, String> {
    if let Some(action) = routine.action {
        return Ok(action);
    }
    let why = match routine.kind {
        RoutineKind::Bound => "it is bound: its events run it",
        RoutineKind::Action => "it does nothing Groove knows",
    };
    Err(format!("`{}` cannot run: {why}", routine.id))
}
