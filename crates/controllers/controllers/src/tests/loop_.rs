use crate::{AppState, Event, Services, Spawner, SyncSpawner, Window, apply};

#[test]
fn a_window_focus_event_lands_in_the_state() {
    let mut state = AppState::default();
    assert!(!state.focused);
    apply(Event::Window(Window::Focus(true)), &mut state);
    assert!(state.focused);
    apply(Event::Window(Window::Focus(false)), &mut state);
    assert!(!state.focused);
}

#[test]
fn a_job_runs_and_its_continuation_writes_the_state() {
    let spawner = SyncSpawner::new().unwrap();
    let mut state = AppState::default();
    spawner.spawn(Box::pin(async {
        let answer = 2 + 2;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.focused = answer == 4
        }) as _
    }));
    assert!(!state.focused, "nothing applies before drain");
    let home = tempfile::tempdir().unwrap();
    let services = spawner.block_on(Services::in_memory(home.path())).unwrap();
    spawner.drain(&mut state, &services);
    assert!(state.focused);
}

#[test]
fn the_feed_keeps_the_newest_errors_and_lets_the_oldest_go() {
    let mut state = AppState::default();
    for at in 0..150 {
        state.failed(groove_types::Error::internal(at.to_string()));
    }
    assert_eq!(state.errors.len(), 100);
    assert_eq!(state.errors[0].message, "50");
    assert_eq!(state.errors[99].message, "149");
}
