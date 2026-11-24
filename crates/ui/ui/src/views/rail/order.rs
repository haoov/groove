//! The rail's order: each section as the user sorted it, each child right under its parent.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::Section;

/// The sections folded to their heading.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Shut {
    active: bool,
    paused: bool,
}

impl Shut {
    pub fn holds(self, section: Section) -> bool {
        match section {
            Section::Active => self.active,
            Section::Paused => self.paused,
        }
    }

    pub fn toggle(&mut self, section: Section) {
        match section {
            Section::Active => self.active = !self.active,
            Section::Paused => self.paused = !self.paused,
        }
    }
}

/// One line of the rail's list.
pub(crate) enum Entry<'a> {
    /// A section's heading and how many sessions stand at its top level.
    Heading(Section, usize),
    Session(&'a Open),
    /// A session spawned by the one above it; `last` when no sibling follows it.
    Child {
        open: &'a Open,
        last: bool,
    },
}

/// The rail's lines: a heading over each section that holds sessions, or over both while
/// `dragging`, each session followed by its children; a shut section its heading alone.
pub(crate) fn entries(app: &AppState, dragging: bool, shut: Shut) -> Vec<Entry<'_>> {
    let mut out = Vec::new();
    for section in Section::ALL {
        let tops = app.session.tops(section);
        if tops.is_empty() && !dragging {
            continue;
        }
        out.push(Entry::Heading(section, tops.len()));
        if shut.holds(section) {
            continue;
        }
        for top in tops {
            out.push(Entry::Session(top));
            let children = app.session.children_of(&top.session.id);
            let count = children.len();
            let lines = children.into_iter().enumerate();
            out.extend(lines.map(|(at, open)| Entry::Child {
                open,
                last: at + 1 == count,
            }));
        }
    }
    out
}

/// Every session the rail shows, in the order drawn.
pub(crate) fn shown(app: &AppState, shut: Shut) -> Vec<&Open> {
    entries(app, false, shut)
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Heading(..) => None,
            Entry::Session(open) | Entry::Child { open, .. } => Some(open),
        })
        .collect()
}
