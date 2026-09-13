//! The task capability. Its slice of `AppState`, the operations on it, its events.

use groove_types::{Task, TaskDates};

/// The `task` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub tasks: Vec<Task>,
    pub dates: Vec<(String, TaskDates)>,
}

/// What the outside world tells this capability.
#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
