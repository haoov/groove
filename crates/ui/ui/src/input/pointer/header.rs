//! What the session header's own buttons do: the task's menu, and finishing it.

use groove_controllers::Command;

use crate::Ui;
use crate::base::hit::{Hits, Target};

/// The rest of the task's actions, under the caret that opened them.
pub(super) fn task_menu(
    ui: &mut Ui,
    hits: &Hits,
    session: groove_types::SessionId,
) -> Vec<Command> {
    let at = hits
        .rect_of(&Target::TaskActions(session.clone()))
        .map(|caret| (caret.x, caret.bottom()))
        .unwrap_or_default();
    ui.overlay = Some(crate::Overlay::Menu(crate::Menu {
        at,
        corner: crate::Corner::TopLeft,
        of: crate::Of::Session(session),
    }));
    Vec::new()
}

/// The task done at its source, and its session taken away.
pub(super) fn finishing(session: groove_types::SessionId) -> Vec<Command> {
    let finish = groove_controllers::task::Command::Finish { session };
    vec![Command::Task(finish)]
}
