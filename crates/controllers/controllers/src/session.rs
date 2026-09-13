//! The `session` controller: one function per user action on the `session` service.

use crate::{AppState, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {}

impl Command {
    pub fn id(&self) -> &'static str {
        match *self {}
    }
}

pub fn dispatch(command: Command, _state: &mut AppState, _spawner: &dyn Spawner) {
    match command {}
}
