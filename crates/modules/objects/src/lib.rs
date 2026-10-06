//! A cluster's kinds, cached on disk, and its objects as Table rows: listed, then watched.

mod discovery;
mod watch;

#[cfg(test)]
mod tests;

pub use discovery::kinds;
pub use watch::{Batch, Delta, Stop, watch};

use groove_types::{KubeKind, ObjectRow, TableColumn};

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
    }
}

fn row_of(row: groove_kube::Row) -> ObjectRow {
    ObjectRow {
        uid: row.uid,
        name: row.name,
        namespace: row.namespace,
        version: row.version,
        cells: row.cells,
    }
}
