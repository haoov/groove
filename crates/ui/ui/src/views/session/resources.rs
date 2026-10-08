//! The Resources tab: the kinds of what the session holds, and the list of the one picked.

mod list;
mod scope;
mod sidebar;

pub use list::draw;
pub use scope::{Dragged, Pick, ResourcesUi, contexts, focus, held, kind, offered, wants};
pub use sidebar::{Line, draw as sidebar, shown};
