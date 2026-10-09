//! The chords the Resources tab hears while the workspace has the keys.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::views::session::Tab;
use crate::views::session::resources::Scoping;
use crate::{Focus, Overlay, Ui};

/// The list's find bar, the kind search, or the namespace picker; `None` for any other chord.
pub(super) fn chord(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Option<Vec<Command>> {
    let (open, true) = (app.session.selected()?, ui.focus == Focus::Workspace) else {
        return None;
    };
    let up = ui.session.tab == Tab::Resources && !open.clusters.is_empty();
    let held = &mut ui.session.resources;
    match () {
        _ if up && held.showing.is_none() && keymap.is(Action::Find, key, mods) => {
            (held.finding, held.typing, held.filtering) = (true, true, false);
        }
        _ if up && keymap.is(Action::OpenPath, key, mods) => {
            (held.filtering, held.typing) = (true, false);
        }
        _ if keymap.is(Action::SelectNamespace, key, mods) => {
            let (scoping, commands) = Scoping::asked(open).opened(open, held);
            ui.overlay = Some(Overlay::Scope(scoping));
            return Some(commands);
        }
        _ => return None,
    }
    Some(Vec::new())
}
