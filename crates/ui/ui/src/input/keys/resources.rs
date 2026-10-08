//! The chords the Resources tab hears while the workspace has the keys.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::palette::{Flow, Palette};
use crate::views::session::Tab;
use crate::{Focus, Overlay, Ui};

/// The list's find bar, the kind search, or the namespace picker; `None` for any other chord.
pub(super) fn chord(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Option<Vec<Command>> {
    let open = app
        .session
        .selected()
        .filter(|open| !open.clusters.is_empty());
    let (open, true) = (open?, ui.focus == Focus::Workspace) else {
        return None;
    };
    let up = ui.session.tab == Tab::Resources;
    let held = &mut ui.session.resources;
    match () {
        _ if up && keymap.is(Action::Find, key, mods) => {
            (held.finding, held.typing, held.filtering) = (true, true, false);
        }
        _ if up && keymap.is(Action::OpenPath, key, mods) => {
            (held.filtering, held.typing) = (true, false);
        }
        _ if keymap.is(Action::SelectNamespace, key, mods) => {
            let flow = Flow::new(
                crate::palette::Action::SelectNamespace,
                open.session.id.clone(),
            );
            ui.overlay = Some(Overlay::Palette(Palette {
                flow: Some(flow),
                ..Palette::default()
            }));
        }
        _ => return None,
    }
    Some(Vec::new())
}
