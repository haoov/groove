//! The session field that has the keys, while its tab is shown.

use groove_controllers::AppState;

use crate::views::session::diff::noting_drawn;
use crate::views::session::resources::View;
use crate::views::session::{Tab, Term};
use crate::{Focus, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Typed {
    Naming,
    Noting,
    Bar(Term),
    Search,
    /// The find bar of an Application's resources.
    ArgoSearch,
}

/// The field typed in, while it is drawn; a hidden one keeps its text.
pub(super) fn keyed(ui: &Ui, app: &AppState) -> Option<Typed> {
    let session = &ui.session;
    let tab = session.tab;
    let code = matches!(tab, Tab::Diff | Tab::Files);
    let resources = tab == Tab::Resources;
    let side = session.sidebar();
    let naming = matches!(ui.focus, Focus::Workspace | Focus::Sidebar);
    let noting = ui.focus == Focus::Workspace;
    match () {
        _ if naming && side && session.naming.is_some() && tab == Tab::Files => Some(Typed::Naming),
        _ if noting && code && noting_drawn(app, ui) => Some(Typed::Noting),
        _ if side && code && session.bar.typing.is_some() => session.bar.typing.map(Typed::Bar),
        _ if resources && session.resources.typing && session.resources.tab().is_none() => {
            Some(Typed::Search)
        }
        _ if resources && argo_typing(ui) => Some(Typed::ArgoSearch),
        _ => None,
    }
}

/// An Application's resources shown, their find bar typed in.
fn argo_typing(ui: &Ui) -> bool {
    let tab = ui.session.resources.tab();
    tab.is_some_and(|one| one.view == View::Resources && one.argo.typing)
}
