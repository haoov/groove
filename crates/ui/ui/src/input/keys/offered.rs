//! The keys of the completion list while it stands under the caret.

use groove_controllers::{AppState, Command, workspace};

use super::super::{Key, Modifiers};
use crate::Ui;
use crate::views::session::diff::completion::{accepted, shown};

/// Up and down choose, tab types the chosen item, escape puts the list away; `None` for another key.
pub(super) fn in_list(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
) -> Option<Vec<Command>> {
    if mods.ctrl || mods.alt || mods.shift {
        return None;
    }
    let shown = shown(app, ui)?;
    let count = shown.items.len();
    let chosen = match key {
        Key::Down => (shown.chosen + 1) % count,
        Key::Up => (shown.chosen + count - 1) % count,
        Key::Tab => {
            let edits = accepted(&shown).into_iter();
            return Some(
                edits
                    .map(|one| Command::Workspace(workspace::Command::Edit(one)))
                    .collect(),
            );
        }
        Key::Escape => return Some(vec![Command::Workspace(workspace::Command::Dismiss)]),
        _ => return None,
    };
    ui.session.chosen = (shown.asked, chosen);
    Some(Vec::new())
}
