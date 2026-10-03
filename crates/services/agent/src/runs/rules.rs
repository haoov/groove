//! Which routines an event starts, and where: once a session, not the selected one, auto-approve on.

use groove_types::{Action, Routine, RoutineKind, RoutinesConfig, SessionId};

use super::{Fired, Run, Runs};

/// One open session as the rules weigh it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub id: SessionId,
    /// The routine it runs, for a routine's own session.
    pub routine: Option<String>,
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

/// A bound routine runs on the open session the event is about; a standalone one in its own.
pub fn target(
    routine: &Routine,
    about: Option<&SessionId>,
    sessions: &Sessions,
) -> Option<SessionId> {
    let open = sessions.open.iter();
    let place = match routine.kind {
        RoutineKind::Standalone => open
            .clone()
            .find(|one| one.routine.as_deref() == Some(&routine.id)),
        RoutineKind::Bound => open
            .clone()
            .find(|one| Some(&one.id) == about && one.routine.is_none()),
        RoutineKind::Action => None,
    };
    place.map(|one| one.id.clone())
}
