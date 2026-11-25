//! A write a list row asks before it lands: a scale, a restart, a delete, or an Application's sync.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster};
use groove_gfx::{Edges, Rect};
use groove_types::SessionId;

use super::{Link, ResourcesUi};
use crate::ctx::Ctx;

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
    Delete,
    /// An Application synced through Argo CD, deleting what left its sources when `prune`.
    Sync {
        prune: bool,
    },
    /// Runs at once: it changes nothing live.
    Refresh {
        hard: bool,
    },
}

impl Write {
    /// The word on its menu row and on its answer.
    pub fn act(self) -> &'static str {
        match self {
            Write::Restart => "restart",
            Write::Scale(_) => "scale",
            Write::Delete => "delete",
            Write::Sync { prune: false } => "sync",
            Write::Sync { prune: true } => "sync with prune",
            Write::Refresh { hard: false } => "refresh",
            Write::Refresh { hard: true } => "hard refresh",
        }
    }

    pub fn immediate(self) -> bool {
        matches!(self, Write::Refresh { .. })
    }
}

impl Written {
    /// `restart api?`, or `scale api to`, the count after it.
    pub fn question(&self) -> String {
        match self.write {
            Write::Restart => format!("restart {}?", self.link.name),
            Write::Scale(_) => format!("scale {} to", self.link.name),
            Write::Delete => format!("delete {}?", self.link.name),
            Write::Sync { prune: false } => format!("sync {}?", self.link.name),
            Write::Sync { prune: true } => format!("sync {} and prune?", self.link.name),
            Write::Refresh { .. } => format!("refresh {}?", self.link.name),
        }
    }

    pub fn command(self) -> Command {
        let (session, link) = (self.session, self.link);
        let asked = match self.write {
            Write::Sync { prune } => cluster::Command::ArgoSync {
                session,
                app: link.pod(),
                prune,
            },
            Write::Refresh { hard } => cluster::Command::ArgoRefresh {
                app: link.pod(),
                hard,
            },
            Write::Restart => cluster::Command::Restart {
                session,
                target: target(link),
            },
            Write::Scale(replicas) => cluster::Command::Scale {
                session,
                target: target(link),
                replicas,
            },
            Write::Delete => cluster::Command::Delete {
                session,
                target: target(link),
            },
        };
        Command::Cluster(asked)
    }
}

fn target(link: Link) -> Box<groove_types::Target> {
    Box::new(groove_types::Target {
        context: link.context,
        kind: link.kind,
        namespace: link.namespace,
        name: link.name,
        subresource: None,
    })
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

/// The question across `line`, a scale's count stepped, then the answers; a sync's prune toggled last.
pub fn ask(ctx: &mut Ctx, line: Rect, written: &Written) {
    use crate::hit::Target;
    let count = match written.write {
        Write::Scale(count) => Some(count),
        _ => None,
    };
    let shown = count.map(|count| count.to_string()).unwrap_or_default();
    let steps = count.map(|count| groove_ui_kit::widgets::Steps {
        shown: &shown,
        less: (count > 0).then_some(Target::Step(-1)),
        more: Target::Step(1),
    });
    let (yes, question) = (written.write.act(), written.question());
    let answers = [(yes, Target::Discard), ("keep", Target::Keep)];
    groove_ui_kit::widgets::question(ctx, line, (&question, steps), answers);
    if let Write::Sync { prune } = written.write {
        let md = ctx.tokens.md;
        let mut room = line.pad(Edges::across(md, md));
        let role = groove_ui_kit::base::style::Role::Bad;
        let toggle = groove_ui_kit::widgets::Toggle::new(prune, Target::ArgoPrune);
        toggle.label("prune").role(role).right(ctx, &mut room, 0.0);
    }
}
