//! The session field that has the keys, while its tab is shown.

use crate::views::session::{SessionUi, Tab, Term};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Typed {
    Naming,
    Noting,
    Bar(Term),
    Kinds,
    Search,
}

/// The field typed in, when the tab that draws it is up; a hidden one keeps its text.
pub(super) fn keyed(session: &SessionUi) -> Option<Typed> {
    let tab = session.tab;
    let code = matches!(tab, Tab::Diff | Tab::Files);
    let resources = tab == Tab::Resources;
    match () {
        _ if session.naming.is_some() && tab == Tab::Files => Some(Typed::Naming),
        _ if session.noting.is_some() && code => Some(Typed::Noting),
        _ if code && session.bar.typing.is_some() => session.bar.typing.map(Typed::Bar),
        _ if resources && session.resources.filtering => Some(Typed::Kinds),
        _ if resources && session.resources.typing => Some(Typed::Search),
        _ => None,
    }
}
