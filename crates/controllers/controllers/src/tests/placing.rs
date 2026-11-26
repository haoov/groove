//! A session placed on the rail by hand: kept in the slice and on disk.

use groove_types::{Section, SessionId};

use crate::session::Command;
use crate::tests::fixture::{self, send};
use crate::{AppState, Services, SyncSpawner};

/// Where the store keeps `id`, read back as a restart would.
fn stored(spawner: &SyncSpawner, services: &Services, id: &SessionId) -> (Section, Option<i64>) {
    let rows = spawner.block_on(services.session.opened()).unwrap();
    let (_, state) = rows.into_iter().find(|(one, _)| &one.id == id).unwrap();
    (state.section, state.rank)
}

fn placed(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    id: &SessionId,
    section: Section,
    index: usize,
) {
    let command = Command::Place {
        session: id.clone(),
        section,
        index,
    };
    send(state, services, spawner, crate::Command::Session(command));
}

#[test]
fn a_new_session_lands_on_top_of_active() {
    let (_home, spawner, services, mut state) = fixture::fresh();
    let first = fixture::explorer(&mut state, &services, &spawner);
    let second = fixture::explorer(&mut state, &services, &spawner);
    let tops: Vec<_> = state
        .session
        .tops(Section::Active)
        .iter()
        .map(|one| one.session.id.clone())
        .collect();
    assert_eq!(tops, [second, first]);
}

#[test]
fn pausing_moves_the_session_and_keeps_it_on_disk() {
    let (_home, spawner, services, mut state) = fixture::fresh();
    let first = fixture::explorer(&mut state, &services, &spawner);
    let second = fixture::explorer(&mut state, &services, &spawner);
    placed(&mut state, &services, &spawner, &first, Section::Paused, 0);
    assert_eq!(state.session.tops(Section::Paused).len(), 1);
    assert_eq!(
        stored(&spawner, &services, &first),
        (Section::Paused, Some(0))
    );

    placed(&mut state, &services, &spawner, &first, Section::Active, 1);
    let tops: Vec<_> = state
        .session
        .tops(Section::Active)
        .iter()
        .map(|one| one.session.id.clone())
        .collect();
    assert_eq!(tops, [second.clone(), first.clone()]);
    assert_eq!(
        stored(&spawner, &services, &first),
        (Section::Active, Some(1))
    );
}

#[test]
fn a_move_within_its_section_reorders_it() {
    let (_home, spawner, services, mut state) = fixture::fresh();
    let first = fixture::explorer(&mut state, &services, &spawner);
    let second = fixture::explorer(&mut state, &services, &spawner);
    placed(&mut state, &services, &spawner, &first, Section::Active, 0);
    let tops: Vec<_> = state
        .session
        .tops(Section::Active)
        .iter()
        .map(|one| one.session.id.clone())
        .collect();
    assert_eq!(tops, [first, second]);
}
