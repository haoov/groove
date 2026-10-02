//! The preview tab: the one the next file opened takes the place of, until it is kept.

use groove_types::{Edit, WorktreeId};

use crate::{Changes, State, from_text};

fn state() -> State {
    let mut state = State::default();
    state.loaded(WorktreeId::new("w1"), Vec::new(), Changes::default());
    state
}

fn open(state: &mut State, path: &str) {
    let file = from_text(path, "one\n", "one\n");
    state.arrived(&WorktreeId::new("w1"), file, None, true);
}

fn tabs(state: &State) -> Vec<(String, bool)> {
    let buffers = state.buffers().expect("the worktree's buffers");
    let all = buffers.all().iter();
    all.map(|one| (one.path.clone(), buffers.previews(&one.path)))
        .collect()
}

#[test]
fn a_file_opened_takes_the_preview_s_place() {
    let mut state = state();
    open(&mut state, "a.rs");
    open(&mut state, "b.rs");
    assert_eq!(tabs(&state), [("b.rs".into(), true)]);
    assert_eq!(state.active().map(|one| one.path.as_str()), Some("b.rs"));
}

#[test]
fn a_kept_file_stays_beside_the_next_preview() {
    let mut state = state();
    open(&mut state, "a.rs");
    state.keep("a.rs");
    open(&mut state, "b.rs");
    open(&mut state, "c.rs");
    assert_eq!(
        tabs(&state),
        [("a.rs".into(), false), ("c.rs".into(), true)]
    );
}

#[test]
fn a_file_kept_before_its_read_came_back_opens_kept() {
    let mut state = state();
    state.keep("a.rs");
    open(&mut state, "a.rs");
    assert_eq!(tabs(&state), [("a.rs".into(), false)]);
}

#[test]
fn the_first_edit_keeps_the_preview() {
    let mut state = state();
    open(&mut state, "a.rs");
    state.edit(&Edit::Insert("x".into()));
    open(&mut state, "b.rs");
    assert_eq!(
        tabs(&state),
        [("a.rs".into(), false), ("b.rs".into(), true)]
    );
}

#[test]
fn opening_a_file_already_in_a_tab_leaves_the_tabs_alone() {
    let mut state = state();
    open(&mut state, "a.rs");
    state.keep("a.rs");
    open(&mut state, "b.rs");
    open(&mut state, "a.rs");
    assert_eq!(
        tabs(&state),
        [("a.rs".into(), false), ("b.rs".into(), true)]
    );
}
