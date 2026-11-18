//! A write a list row asks before it lands: a restart, or a scale to a count.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster};
use groove_types::SessionId;

use super::{Link, ResourcesUi};

/// The write a row of the list asks, the row by its uid.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    pub session: SessionId,
    pub uid: String,
    pub link: Link,
    pub write: Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Write {
    Restart,
    Scale(u32),
}

impl Write {
    /// The word on its menu row and on its answer.
    pub fn act(self) -> &'static str {
        match self {
            Write::Restart => "restart",
            Write::Scale(_) => "scale",
        }
    }
}

impl Written {
    /// `restart api?`, or `scale api to`, the count after it.
    pub fn question(&self) -> String {
        match self.write {
            Write::Restart => format!("restart {}?", self.link.name),
            Write::Scale(_) => format!("scale {} to", self.link.name),
        }
    }

    pub fn command(self) -> Command {
        let link = self.link;
        let target = Box::new(groove_types::Target {
            context: link.context,
            kind: link.kind,
            namespace: link.namespace,
            name: link.name,
            subresource: None,
        });
        let session = self.session;
        let asked = match self.write {
            Write::Restart => cluster::Command::Restart { session, target },
            Write::Scale(replicas) => cluster::Command::Scale {
                session,
                target,
                replicas,
            },
        };
        Command::Cluster(asked)
    }
}

/// The count a row's ready cell wants, `2` of `1/2`.
pub fn desired(app: &AppState, open: &Open, held: &ResourcesUi, uid: &str) -> Option<u32> {
    super::scope::keys(app, open, held).iter().find_map(|key| {
        let watched = app.cluster.store.watched(key)?;
        let ready = watched.columns.iter().position(|one| one.name == "Ready")?;
        let row = watched.rows.iter().find(|one| one.uid == uid)?;
        let (_, wanted) = row.cells.get(ready)?.split_once('/')?;
        wanted.trim().parse().ok()
    })
}
