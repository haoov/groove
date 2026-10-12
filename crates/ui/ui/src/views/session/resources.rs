//! The Resources tab: the kinds of what the session holds, and the list of the one picked.

mod argo;
mod around;
mod list;
mod logs;
mod opened;
mod scope;
mod scoping;
mod sidebar;
mod tab;

pub use argo::{ArgoUi, Filter};
pub use around::row_link;
pub use list::draw;
pub use logs::{LogsUi, RANGES, View, first as first_container, log_key};
pub use opened::{Link, Opens, Section};
pub use scope::{Dragged, Pick, ResourcesUi, contexts, held, kind, offered, wants};
pub use scoping::{How, ScopeLine, Scoping, choose, lines as scope_lines};
pub use sidebar::{Line, draw as sidebar, shown};
pub use tab::{due as tab_due, wants as tab_wants};
