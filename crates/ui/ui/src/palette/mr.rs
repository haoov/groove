//! The MR's own actions, as the palette's Git group offers them.

use groove_controllers::delivery::{Command as Mr, Say};
use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command};
use groove_types::{ReviewVerdict, SessionKind, Worktree};

use super::Entry;

pub(super) fn entries(app: &AppState, open: &Open, worktree: &Worktree) -> Vec<Entry> {
    let mr = |label: &str, one: Mr| Entry::command("MR", label, Command::Delivery(one));
    let review = |verdict| Mr::Say(Say::Review(verdict));
    let delivery = app.delivery.row(&worktree.id, open.status_of(&worktree.id));
    let Some(held) = delivery.mr else {
        return vec![mr("Create the MR", Mr::CreateMr)];
    };
    let mut out = vec![
        mr("Open the MR in the browser", Mr::BrowseMr { url: held.url }),
        mr("Read the MR again", Mr::RefreshMr),
        mr("Comment on the MR", review(ReviewVerdict::Comment)),
    ];
    match open.session.kind {
        SessionKind::Review { .. } => out.extend([
            mr("Approve the MR", review(ReviewVerdict::Approve)),
            mr("Request changes", review(ReviewVerdict::RequestChanges)),
        ]),
        _ => out.push(mr("Update the MR", Mr::UpdateMr)),
    }
    out.push(mr("Close the MR", Mr::CloseMr));
    out
}
