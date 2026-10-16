//! The Resources tab: the kinds of what the session holds, and the list of the one picked.

mod argo;
mod around;
mod health;
mod list;
mod logs;
mod opened;
mod scope;
mod scoping;
mod sidebar;
mod tab;

pub use argo::{ArgoUi, Filter};
pub use around::row_link;
pub use list::{draw, uids as row_uids};
pub use logs::{LogsUi, RANGES, View, first as first_container, log_key};
pub use opened::{Link, Opens, Section};
pub use scope::{
    Dragged, Pick, ResourcesUi, WHOLE_LINE, WHOLE_MARK, WHOLE_NAME, contexts, held, kind, offered,
    shown_contexts, wants,
};
pub use scoping::{How, ScopeLine, Scoping, choose, lines as scope_lines};
pub use sidebar::{Line, draw as sidebar, shown};
pub use tab::{due as tab_due, wants as tab_wants};

use groove_controllers::{Command, cluster};

/// Nothing while `reader` holds what it wants; else let go of all it holds and take `reads`.
fn leases(reader: &str, same: bool, reads: impl Iterator<Item = Command>) -> Vec<Command> {
    if same {
        return Vec::new();
    }
    let release = cluster::Command::Release {
        reader: reader.into(),
    };
    std::iter::once(Command::Cluster(release))
        .chain(reads)
        .collect()
}
