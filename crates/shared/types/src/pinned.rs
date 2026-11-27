//! A list a session pins: a kind, and the contexts and namespaces it reads.

use crate::{Attached, KubeKind};

/// The kind and the scope of a list, a click away under `LISTS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// The kind as stored: no short names, watchable.
    pub kind: KubeKind,
    /// The pairs read, sorted, each once.
    pub scope: Vec<Attached>,
}

impl Pin {
    pub fn new(kind: &KubeKind, scope: impl IntoIterator<Item = Attached>) -> Self {
        let mut scope: Vec<Attached> = scope.into_iter().collect();
        scope.sort();
        scope.dedup();
        let kind = KubeKind {
            short_names: Vec::new(),
            watchable: true,
            ..kind.clone()
        };
        Self { kind, scope }
    }

    /// The contexts it reads, each once, in order.
    pub fn contexts(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.scope.iter().map(|one| one.context.as_str()).collect();
        out.dedup();
        out
    }
}
