use crate::{AppState, Event, Spawner, SyncSpawner, Window, apply};

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
        Box::new(move |state: &mut AppState| state.focused = answer == 4) as _
    }));
    assert!(!state.focused, "nothing applies before drain");
    spawner.drain(&mut state);
    assert!(state.focused);
}
