//! The chords that pick a tab: a mode while the modes layout is on, else the workspace's tab.

use groove_controllers::AppState;

use crate::keymap::Action;
use crate::views::session::Tab;
use crate::{Focus, Ui};

/// The tab `action` names; the agent's without modes hands the keys to the agent.
pub(super) fn tabbed(action: Action, ui: &mut Ui, app: &AppState) {
    use crate::views::session::Mode;
    let mode = match action {
        Action::Overview => Mode::Overview,
        Action::AgentTab => Mode::Agent,
        Action::Diff => Mode::Diff,
        Action::Files => Mode::Files,
        _ => Mode::Kubernetes,
    };
    let clusters = app
        .session
        .selected()
        .is_some_and(|one| one.holds_clusters());
    if mode == Mode::Kubernetes && !clusters {
        return;
    }
    if ui.modes {
        return (ui.session.mode, ui.aside) = (mode, 0.0);
    }
    ui.session.tab = match mode {
        Mode::Agent => return ui.focus = Focus::Agent,
        Mode::Overview => Tab::Overview,
        Mode::Diff => Tab::Diff,
        Mode::Files => Tab::Files,
        Mode::Kubernetes => Tab::Resources,
    };
}
