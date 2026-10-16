//! The open files' and objects' tabs from the keyboard: the next, the previous, the one closed.

use groove_controllers::{AppState, Command};

use crate::Ui;
use crate::hit::{Hits, Target};
use crate::keymap::{Action, Keymap};
use crate::views::session::Tab;
use groove_ui_kit::base::ctx::Metrics;

use super::super::{Key, Modifiers};

/// The chord's action on the open tabs of the Files or the Kubernetes tab; `None` for another.
pub(super) fn chord(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (keymap, seen): (&Keymap, (&Hits, Metrics)),
) -> Option<Vec<Command>> {
    let action = [Action::CloseOpen, Action::NextOpen, Action::PreviousOpen]
        .into_iter()
        .find(|one| keymap.is(*one, key, mods))?;
    match ui.session.tab {
        Tab::Files => Some(files(action, ui, app, seen)),
        Tab::Resources => {
            objects(action, ui);
            Some(Vec::new())
        }
        _ => None,
    }
}

/// The open files: a click on the tab beside the active one, or on the active one's cross.
fn files(action: Action, ui: &mut Ui, app: &AppState, seen: (&Hits, Metrics)) -> Vec<Command> {
    let open: Vec<String> = app
        .workspace
        .buffers()
        .map(|one| one.all().iter().map(|file| file.path.clone()).collect())
        .unwrap_or_default();
    let active = app.workspace.active().map(|one| one.path.clone());
    let Some(at) = open.iter().position(|path| Some(path) == active.as_ref()) else {
        return Vec::new();
    };
    let target = match action {
        Action::CloseOpen => Target::CloseTab(open[at].clone()),
        Action::NextOpen => Target::OpenTab(open[(at + 1) % open.len()].clone()),
        _ => Target::OpenTab(open[(at + open.len() - 1) % open.len()].clone()),
    };
    crate::input::pointer::act_on(target, ui, app, seen)
}

/// The list and the objects open beside it: the list counts as the first tab.
fn objects(action: Action, ui: &mut Ui) {
    let held = &mut ui.session.resources;
    let len = held.opened.len() + 1;
    let at = held.showing.map_or(0, |one| one + 1);
    let next = match action {
        Action::CloseOpen => {
            if let Some(shown) = held.showing {
                held.close(shown);
            }
            return;
        }
        Action::NextOpen => (at + 1) % len,
        _ => (at + len - 1) % len,
    };
    (held.showing, held.typing) = (next.checked_sub(1), false);
}
