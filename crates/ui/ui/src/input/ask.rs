//! The agent asked about something seen elsewhere: where it stands, put in the prompt, the keys moved there.

use groove_types::{Anchor, Edit};

use crate::{Focus, Ui};

/// `about` at the end of the prompt, for the user to finish and send.
pub(crate) fn asked(ui: &mut Ui, about: String) {
    ui.agent.prompt.edit(&Edit::Insert(about));
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
