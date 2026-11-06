//! What lands in the agent's prompt from elsewhere: a command offered after `/`, or where lines stand.

use groove_types::{Anchor, Edit};

use crate::{Focus, Ui};

/// `about` at the end of the prompt, for the user to finish and send.
pub(crate) fn asked(ui: &mut Ui, about: String) {
    ui.agent.prompt.edit(&Edit::Insert(about));
    ui.focus = Focus::Agent;
}

/// The command `name` in the prompt, ready for its arguments.
pub(crate) fn command(ui: &mut Ui, name: &str) {
    ui.agent.prompt = Default::default();
    ui.agent.prompt.edit(&Edit::Insert(format!("/{name} ")));
    ui.agent.offered = 0;
    ui.focus = Focus::Agent;
}

/// `` `src/lib.rs:12-14` ``, its lines counted from one.
pub(crate) fn place(anchor: &Anchor) -> String {
    let (start, end) = (anchor.start_line + 1, anchor.end_line + 1);
    match start == end {
        true => format!("`{}:{start}`", anchor.path),
        false => format!("`{}:{start}-{end}`", anchor.path),
    }
}
