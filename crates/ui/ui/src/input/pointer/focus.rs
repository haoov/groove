//! Which pane a click lands in.

use crate::hit::Target;
use crate::{Focus, Ui};

/// The pane a click lands in, the search bars let go of the keys when it is none of theirs.
pub(super) fn focused(target: &Option<Target>, ui: &mut Ui) {
    ui.focus = pane(target, ui.focus);
    if matches!(ui.focus, Focus::Agent | Focus::Terminal | Focus::Rail) {
        ui.session.bar.typing = None;
        if let Some(find) = ui.session.find.as_mut() {
            find.typing = false;
        }
        (ui.session.resources.typing, ui.session.resources.filtering) = (false, false);
    }
}

/// The pane a click lands in. What it lands on says which.
fn pane(target: &Option<Target>, focus: Focus) -> Focus {
    match target {
        Some(Target::Session(_) | Target::FeedLine(_)) => Focus::Rail,
        Some(Target::Agent) => Focus::Agent,
        Some(Target::Shell(_)) => Focus::Terminal,
        Some(Target::Code) | Some(Target::View(_)) | Some(Target::Mode(_)) => Focus::Workspace,
        Some(
            Target::Tab(_)
            | Target::ResourceList
            | Target::ResourceRow(_)
            | Target::ResourceSearch
            | Target::ResourceFilter
            | Target::ResourceKind(_)
            | Target::ResourceGroup(_)
            | Target::ResourceSort(_)
            | Target::ResourceTab(_)
            | Target::ResourceClose(_)
            | Target::ResourceView(_)
            | Target::ResourceContainer(_)
            | Target::ResourceSection(_)
            | Target::ResourceOpen(_)
            | Target::ResourceCopy(..)
            | Target::LogSource(_)
            | Target::LogPrevious(_)
            | Target::LogRange
            | Target::LogFollow,
        ) => Focus::Workspace,
        Some(
            Target::File(_)
            | Target::Stage(_)
            | Target::Unstage(_)
            | Target::Discard
            | Target::Keep
            | Target::Message
            | Target::Do,
        ) => Focus::Sidebar,
        _ => focus,
    }
}
