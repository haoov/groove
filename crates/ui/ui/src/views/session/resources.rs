//! The Resources tab: the kinds of what the session holds, and the list of the one picked.

mod argo;
mod list;
mod logs;
mod opened;
mod scope;
mod scoping;
mod sidebar;
mod tab;

pub use argo::{ArgoUi, Filter};
pub use list::draw;
pub use logs::{LogsUi, RANGES, View, first as first_container, log_key};
pub use opened::{Link, Section, row_link};
pub use scope::{Dragged, Pick, ResourcesUi, contexts, held, kind, offered, wants};
pub use scoping::{How, ScopeLine, Scoping, choose, lines as scope_lines};
pub use sidebar::{Line, draw as sidebar, shown};
pub use tab::{ARGO_WIDTHS, due as tab_due, wants as tab_wants};
