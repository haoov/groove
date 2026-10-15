//! Which pane a click lands in.

use crate::hit::Target;
use crate::{Focus, Ui};

/// The pane a click lands in, the search bars let go of the keys when it is none of theirs.
pub(super) fn focused(target: &Option<Target>, ui: &mut Ui) {
    ui.focus = pane(target, ui.focus);
    if listed(target) {
        ui.session.composing = false;
    }
    if matches!(ui.focus, Focus::Agent | Focus::Terminal | Focus::Rail) {
        ui.session.bar.typing = None;
        if let Some(find) = ui.session.find.as_mut() {
            find.typing = false;
        }
        (ui.session.resources.typing, ui.session.resources.filtering) = (false, false);
    }
}

/// A row of the sidebar's lists, or what changes the list: the keys leave the commit box.
fn listed(target: &Option<Target>) -> bool {
    matches!(
        target,
        Some(
            Target::File(_)
                | Target::Dir(_)
                | Target::Group(_)
                | Target::Commit(_)
                | Target::NoteAt(_)
                | Target::Pane(_)
                | Target::Showing(_)
        )
    )
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
            | Target::Overview
            | Target::Worktree(_)
            | Target::LogHours(_)
            | Target::Link(_)
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
            | Target::ArgoFilter(_)
            | Target::ResourceFold(..)
            | Target::ContainerUsage(..)
            | Target::ResourceSection(_)
            | Target::ResourceOpen(_)
            | Target::ResourceCopy(..)
            | Target::LogSource(_)
            | Target::LogPrevious(_)
            | Target::LogRange
            | Target::LogFollow,
        ) => Focus::Workspace,
        Some(target) if listed(&Some(target.clone())) => Focus::Sidebar,
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
