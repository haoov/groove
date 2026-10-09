//! The Resources tab: the kinds of what the session holds, and the list of the one picked.

mod list;
mod scope;
mod scoping;
mod sidebar;

pub use list::draw;
pub use scope::{Dragged, Pick, ResourcesUi, contexts, held, kind, offered, wants};
pub use scoping::{How, ScopeLine, Scoping, choose, lines as scope_lines};
pub use sidebar::{Line, draw as sidebar, shown};
