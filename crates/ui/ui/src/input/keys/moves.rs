//! Keys that move rather than type: `ctrl+j` and `ctrl+k` down and up a list, a pane resized.

use groove_controllers::AppState;

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::views::session::Tab;
use crate::{Focus, Overlay, Surface, Ui};

/// The key as an arrow when a list has the keys and the key is bound to a step down or up it.
pub(super) fn arrowed(
    (key, mods): (Key, Modifiers),
    ui: &Ui,
    app: &AppState,
    keymap: &Keymap,
) -> (Key, Modifiers) {
    let step = [(Action::ListDown, Key::Down), (Action::ListUp, Key::Up)]
        .into_iter()
        .find(|(action, _)| keymap.is(*action, key, mods));
    match step {
        Some((_, arrow)) if listing(ui, app) => (arrow, Modifiers::default()),
        _ => (key, mods),
    }
}

/// A palette, a scope panel, the rail, the sidebar, the board or the Kubernetes list.
fn listing(ui: &Ui, app: &AppState) -> bool {
    if matches!(ui.overlay, Some(Overlay::Palette(_) | Overlay::Scope(_))) {
        return true;
    }
    if ui.settings.open || ui.overlay.is_some() {
        return false;
    }
    if ui.showing(app) == Surface::Board {
        return true;
    }
    match ui.focus {
        Focus::Rail | Focus::Sidebar => true,
        Focus::Workspace => {
            ui.session.tab == Tab::Resources && ui.session.resources.tab().is_none()
        }
        Focus::Agent | Focus::Terminal => false,
    }
}

/// What a resize chord asks: across or up and down, and whether the pane grows.
pub(super) fn nudge(action: Action) -> Option<(bool, bool)> {
    match action {
        Action::ResizeLeft => Some((true, false)),
        Action::ResizeRight => Some((true, true)),
        Action::ResizeUp => Some((false, true)),
        Action::ResizeDown => Some((false, false)),
        _ => None,
    }
}
