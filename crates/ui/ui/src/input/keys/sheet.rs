//! The review sheet's keys: Enter approves the write it shows, Escape refuses it.

use groove_controllers::{Command, agent};

use super::super::{Key, Modifiers};
use crate::{Overlay, Ui};

/// The answer a plain Enter or Escape gives the write the sheet shows, the sheet closed.
pub(super) fn answered((key, mods): (Key, Modifiers), ui: &mut Ui) -> Option<Vec<Command>> {
    let id = ui.examining()?.clone();
    if mods != Modifiers::default() {
        return None;
    }
    let answer = match key {
        Key::Enter => agent::Command::Approve { id },
        Key::Escape => agent::Command::Refuse { id },
        _ => return None,
    };
    ui.close(|one| matches!(one, Overlay::Examining(_)));
    Some(vec![Command::Agent(answer)])
}
