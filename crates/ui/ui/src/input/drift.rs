//! An Application's diff view scrolled to the file of the resource it was opened on.

use groove_controllers::AppState;

use crate::Ui;
use crate::views::session::Tab;
use crate::views::session::resources::View;
use groove_ui_kit::base::ctx::Metrics;

/// The rows kept above the file's head.
const ABOVE: usize = 3;

/// The stream on the file waited for, once the drift holds it; another view lets it go.
pub(super) fn landed(ui: &mut Ui, app: &AppState, metrics: Metrics) {
    let shown = ui.session.tab == Tab::Resources;
    let Some(tab) = ui.session.resources.tab_mut() else {
        return;
    };
    let Some(path) = tab.argo.landing.as_ref() else {
        return;
    };
    if !shown || tab.view != View::Diff {
        tab.argo.landing = None;
        return;
    }
    let Some(head) = app.workspace.changes.head_of(path) else {
        return;
    };
    tab.argo.landing = None;
    ui.session.diff = head.saturating_sub(ABOVE) as f32 * metrics.tokens().code_line;
}
