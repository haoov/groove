//! What the store says of an object's relations: the kinds served, its owners, the Services that pick it.

use std::sync::Arc;

use groove_types::{Described, FollowKey, KubeKind};

use super::Store;

/// How far up the owners a lineage reads.
const OWNERS_MAX: usize = 5;

/// One owner of an object, and the owner as last read.
#[derive(Debug, Clone)]
pub struct Up {
    pub kind: KubeKind,
    pub namespace: Option<String>,
    pub name: String,
    pub read: Option<Arc<Described>>,
}

impl Store {
    /// The kind `context` serves in `group` named `kind`.
    pub fn served(&self, context: &str, group: &str, kind: &str) -> Option<KubeKind> {
        let same = |one: &&KubeKind| one.group == group && one.kind == kind;
        self.kinds(context)?.iter().find(same).cloned()
    }

    /// The one object `key` names, once its watcher has listed it.
    pub fn object(&self, key: &FollowKey) -> Option<&Arc<Described>> {
        self.follows.get(key)?.objects.first()
    }

    /// The object's owners in `context`, nearest first, as far as each has been read.
    pub fn lineage(&self, context: &str, namespace: Option<&str>, object: &Described) -> Vec<Up> {
        let mut out: Vec<Up> = Vec::new();
        let mut owners = object.owners.clone();
        while let Some(owner) = owners.first().cloned() {
            let Some(kind) = self.served(context, owner.group(), &owner.kind) else {
                break;
            };
            let namespace = namespace.filter(|_| kind.namespaced).map(String::from);
            let key = FollowKey::named(context, &kind, namespace.as_deref(), &owner.name);
            let read = self.object(&key).cloned();
            owners = read
                .as_ref()
                .map(|one| one.owners.clone())
                .unwrap_or_default();
            out.push(Up {
                kind,
                namespace,
                name: owner.name,
                read,
            });
            if out.len() >= OWNERS_MAX {
                break;
            }
        }
        out
    }

    /// The events about the object `uid`, once `context` serves them.
    pub fn events_key(
        &self,
        context: &str,
        namespace: Option<&str>,
        uid: &str,
    ) -> Option<FollowKey> {
        Some(FollowKey {
            context: context.to_string(),
            kind: self.served(context, "", "Event")?,
            namespace: namespace.map(String::from),
            fields: Some(format!("involvedObject.uid={uid}")),
        })
    }

    /// Every Service of the namespace.
    pub fn services_key(&self, context: &str, namespace: Option<&str>) -> Option<FollowKey> {
        Some(FollowKey {
            context: context.to_string(),
            kind: self.served(context, "", "Service")?,
            namespace: namespace.map(String::from),
            fields: None,
        })
    }

    /// The Services whose selector picks `object` by its labels.
    pub fn selecting(
        &self,
        context: &str,
        namespace: Option<&str>,
        object: &Described,
    ) -> Vec<String> {
        let key = self.services_key(context, namespace);
        let Some(services) = key.and_then(|key| self.follows.get(&key)) else {
            return Vec::new();
        };
        let picks = services.objects.iter().filter(|one| one.selects(object));
        picks.map(|one| one.name.clone()).collect()
    }
}
