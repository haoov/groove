//! The `config` controller: one function per user action on the `config` service.

use crate::{AppState, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {}

impl Command {
    pub fn id(&self) -> &'static str {
        match *self {}
    }
}

pub fn dispatch(
    command: Command,
    _state: &mut AppState,
    _services: &Services,
    _spawner: &dyn Spawner,
) {
    match command {}
}
