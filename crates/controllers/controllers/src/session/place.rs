//! A session put where the user wants it on the rail: its section and its place there.

use groove_types::{Section, SessionId};

use crate::spawn::record;
use crate::{AppState, Services, Spawner};

/// The session at `index` of `section`, kept in the slice and written behind.
pub fn place(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    section: Section,
    index: usize,
) {
    let places = state.session.place(id, section, index);
    if places.is_empty() {
        return;
    }
    let service = services.session.clone();
    record(spawner, async move { service.set_places(&places).await });
}
