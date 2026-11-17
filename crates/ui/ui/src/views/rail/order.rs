//! The rail's order: the sessions by state, each child right under its parent.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;

use super::group_of;

/// One session the rail lists, and whether it stands under its parent.
pub(crate) type Placed<'a> = (&'a Open, bool);

/// The open sessions by state with their children, then the routines' own, in the order opened.
pub(super) fn split(app: &AppState) -> (Vec<Placed<'_>>, Vec<&Open>) {
    let routine = |open: &&Open| open.session.kind.routine().is_none();
    let (sessions, routines): (Vec<&Open>, Vec<&Open>) = app.session.open.iter().partition(routine);
    let under = |open: &&Open| {
        let parent = open.state.parent.as_ref();
        parent.is_some_and(|one| app.session.get(one).is_some())
    };
    let (children, mut tops): (Vec<&Open>, Vec<&Open>) = sessions.into_iter().partition(under);
    tops.sort_by_key(|open| group_of(app, open));
    let mut out = Vec::new();
    for top in tops {
        out.push((top, false));
        let id = Some(&top.session.id);
        let own = children
            .iter()
            .filter(|one| one.state.parent.as_ref() == id);
        out.extend(own.map(|one| (*one, true)));
    }
    (out, routines)
}
