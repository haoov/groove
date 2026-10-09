//! A cluster's kinds, cached on disk, and its objects as Table rows: listed, then watched.

mod describe;
mod discovery;
mod follow;
mod reads;
mod watch;
mod yaml;

#[cfg(test)]
mod tests;

pub use describe::describe;
pub use discovery::{kinds, namespaces};
pub use follow::{Change, Followed, follow};
pub use reads::{helm_revision, usage};
pub use watch::{Batch, Delta, Stop, watch};

use groove_types::{KubeKind, ObjectRow, TableColumn, Timestamp};

fn kind_of(kind: groove_kube::Kind) -> KubeKind {
    KubeKind {
        group: kind.group,
        version: kind.version,
        kind: kind.kind,
        plural: kind.plural,
        namespaced: kind.namespaced,
        watchable: kind.watchable,
    }
}

fn kube_kind(kind: &KubeKind) -> groove_kube::Kind {
    groove_kube::Kind {
        group: kind.group.clone(),
        version: kind.version.clone(),
        kind: kind.kind.clone(),
        plural: kind.plural.clone(),
        namespaced: kind.namespaced,
        watchable: kind.watchable,
    }
}

fn column_of(column: groove_kube::Column) -> TableColumn {
    TableColumn {
        name: column.name,
        priority: column.priority,
        date: column.date,
    }
}

/// The row as it arrived at `received`, its time cells read against `columns`.
fn row_of(row: groove_kube::Row, columns: &[TableColumn], received: Timestamp) -> ObjectRow {
    let created = row.created.and_then(|at| Timestamp::parse(&at).ok());
    ObjectRow {
        aging: groove_types::agings(columns, &row.cells, created, received),
        uid: row.uid,
        name: row.name,
        namespace: row.namespace,
        version: row.version,
        cells: row.cells,
    }
}
