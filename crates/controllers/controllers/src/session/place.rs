//! A session put where the user wants it on the rail: its section and its place there.

use groove_types::{Section, SessionId};

use super::log::{Logged, log_work};
use crate::asker::Asker;
use crate::spawn::record;
use crate::{AppState, Services, Spawner};

/// The session at `index` of `section`, kept in the slice and written behind.
/// A move to another section writes it to the session's log.
pub fn place(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    section: Section,
    index: usize,
) {
    let Some(was) = state.session.get(id).map(|one| one.state.section) else {
        return;
    };
    let places = state.session.place(id, section, index);
    if places.is_empty() {
        return;
    }
    let service = services.session.clone();
    record(spawner, async move { service.set_places(&places).await });
    if was == section {
        return;
    }
    let what = match section {
        Section::Paused => "paused",
        Section::Active => "back to active",
    };
    let logged = Logged {
        what: what.into(),
        ..Logged::default()
    };
    log_work(state, services, spawner, id, logged, Asker::Ui);
}
