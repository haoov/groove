//! The chords that pick the session's mode.

use groove_controllers::AppState;

use crate::Ui;
use crate::keymap::Action;

/// The mode `action` names.
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
    (ui.session.mode, ui.aside) = (mode, 0.0);
}
