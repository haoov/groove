//! What a row says of its session's kind: a tag of capitals, and the number its title leads with.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::{Forge, SessionKind, TaskKey};

/// `TSK`, `EXP`, `MR` or `PR`.
pub(crate) fn tag(app: &AppState, open: &Open) -> &'static str {
    match &open.session.kind {
        SessionKind::Task { .. } => "TSK",
        SessionKind::Explorer => "EXP",
        SessionKind::Review { .. } => match forge(app, open) {
            Forge::Github => "PR",
            Forge::Gitlab => "MR",
        },
    }
}

/// `#148` for a GitHub issue or PR, `!88` for a GitLab MR, a Notion task's own id; none for
/// an explorer.
pub(crate) fn number(app: &AppState, open: &Open) -> Option<String> {
    match &open.session.kind {
        SessionKind::Task { external_id } => match TaskKey::parse(external_id).ok()? {
            TaskKey::Github { number, .. } => Some(format!("#{number}")),
            TaskKey::Notion { .. } => app
                .task
                .by_external(external_id)
                .map(|task| task.short_id.clone()),
        },
        SessionKind::Review { iid, .. } => Some(format!("{}{iid}", forge(app, open).sigil())),
        SessionKind::Explorer => None,
    }
}

/// The forge a review stands on: the queue's word for it, else its repo's host.
fn forge(app: &AppState, open: &Open) -> Forge {
    let SessionKind::Review { project, iid } = &open.session.kind else {
        return Forge::Gitlab;
    };
    let queued = app.delivery.reviews.iter();
    let mut queued = queued.filter(|mr| &mr.project == project && mr.iid == *iid);
    if let Some(mr) = queued.next() {
        return mr.forge;
    }
    open.repos
        .first()
        .map_or(Forge::Gitlab, |repo| Forge::of_host(&repo.host))
}
