//! The objects open in tabs of their own, and what each reads around itself.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_types::{Described, FollowKey, KubeKind};

/// Where an object stands: enough to follow it and to open its tab.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link {
    pub context: String,
    pub kind: KubeKind,
    pub namespace: Option<String>,
    pub name: String,
}

impl Link {
    pub fn key(&self) -> FollowKey {
        FollowKey::named(
            &self.context,
            &self.kind,
            self.namespace.as_deref(),
            &self.name,
        )
    }

    /// The object as last read, once its watcher has listed it.
    pub fn read<'a>(&self, app: &'a AppState) -> Option<&'a Arc<Described>> {
        app.cluster.store.follows.get(&self.key())?.objects.first()
    }
}

/// A part of the described view, which folds under its heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section {
    Operation,
    Conditions,
    Summary,
    Sources,
    Relations,
    Containers,
    Labels,
    Annotations,
    Events,
}

/// One object's tab: which view, which container, how far down.
#[derive(Debug, Clone, PartialEq)]
pub struct Opened {
    pub link: Link,
    pub view: super::View,
    pub logs: super::LogsUi,
    pub container: Option<String>,
    /// The sections folded shut; the annotations and the events until opened.
    pub shut: std::collections::BTreeSet<Section>,
    pub scroll: f32,
    /// How far the YAML is scrolled sideways.
    pub across: f32,
    /// The value last copied from the tab.
    pub copied: Option<String>,
}

impl Opened {
    pub fn new(link: Link) -> Self {
        Self {
            link,
            view: super::View::Describe,
            logs: super::LogsUi::default(),
            container: None,
            shut: [Section::Annotations, Section::Events].into(),
            scroll: 0.0,
            across: 0.0,
            copied: None,
        }
    }
}

impl super::ResourcesUi {
    /// The object's tab shown, opened first unless it is open already.
    pub fn open(&mut self, link: Link) {
        let at = match self.opened.iter().position(|one| one.link == link) {
            Some(at) => at,
            None => {
                self.opened.push(Opened::new(link));
                self.opened.len() - 1
            }
        };
        self.showing = Some(at);
    }

    /// The tab at `at` closed; the one before it shows, or the list.
    pub fn close(&mut self, at: usize) {
        if at >= self.opened.len() {
            return;
        }
        self.opened.remove(at);
        self.showing = match self.showing {
            Some(shown) if shown > at => Some(shown - 1),
            Some(shown) if shown == at => at.checked_sub(1),
            other => other,
        };
    }

    pub fn tab(&self) -> Option<&Opened> {
        self.opened.get(self.showing?)
    }

    pub fn tab_mut(&mut self) -> Option<&mut Opened> {
        self.opened.get_mut(self.showing?)
    }

    /// How far down the panel shown is scrolled: the object's tab, or the list. A scroll of logs stops following.
    pub fn scroll_mut(&mut self) -> &mut f32 {
        let at = self.showing.filter(|at| *at < self.opened.len());
        match at {
            Some(at) => {
                let tab = &mut self.opened[at];
                tab.logs.following &= tab.view != super::View::Logs;
                &mut tab.scroll
            }
            None => &mut self.scroll,
        }
    }

    /// How far down the panel shown is scrolled; past the end while its logs follow.
    pub fn scroll(&self) -> f32 {
        match self.tab() {
            Some(one) if one.view == super::View::Logs && one.logs.following => f32::MAX,
            Some(one) => one.scroll,
            None => self.scroll,
        }
    }

    /// How far the YAML shown is scrolled sideways; the list never is.
    pub fn across(&self) -> f32 {
        self.tab().map_or(0.0, |one| one.across)
    }

    pub fn across_mut(&mut self) -> &mut f32 {
        let at = self.showing.filter(|at| *at < self.opened.len());
        match at {
            Some(at) => &mut self.opened[at].across,
            None => &mut self.scroll,
        }
    }
}

/// The object a row of the list stands for, by its uid.
pub fn row_link(
    app: &AppState,
    open: &groove_controllers::session_service::Open,
    held: &super::ResourcesUi,
    uid: &str,
) -> Option<Link> {
    for key in super::scope::keys(app, open, held) {
        let Some(watched) = app.cluster.store.watched(&key) else {
            continue;
        };
        if let Some(row) = watched.rows.iter().find(|one| one.uid == uid) {
            return Some(Link {
                context: key.context.clone(),
                kind: key.kind.clone(),
                namespace: row.namespace.clone(),
                name: row.name.clone(),
            });
        }
    }
    None
}

/// The kind `context` serves in `group` named `kind`.
pub fn served(app: &AppState, context: &str, group: &str, kind: &str) -> Option<KubeKind> {
    let kinds = app.cluster.store.kinds(context)?;
    let same = |one: &&KubeKind| one.group == group && one.kind == kind;
    kinds.iter().find(same).cloned()
}

/// The object's owners, nearest first, as far as each has been read.
pub fn lineage(
    app: &AppState,
    link: &Link,
    object: &Described,
) -> Vec<(Link, Option<Arc<Described>>)> {
    let mut out: Vec<(Link, Option<Arc<Described>>)> = Vec::new();
    let mut owners = object.owners.clone();
    while let Some(owner) = owners.first().cloned() {
        let Some(kind) = served(app, &link.context, owner.group(), &owner.kind) else {
            break;
        };
        let up = Link {
            context: link.context.clone(),
            kind,
            namespace: link.namespace.clone(),
            name: owner.name.clone(),
        };
        let read = up.read(app).cloned();
        owners = read
            .as_ref()
            .map(|one| one.owners.clone())
            .unwrap_or_default();
        out.push((up, read));
        if out.len() > 4 {
            break;
        }
    }
    out
}

/// The Helm release an object or its topmost owner was installed by: its namespace and name.
pub fn helm_release(
    object: &Described,
    owners: &[(Link, Option<Arc<Described>>)],
) -> Option<(String, String)> {
    let top = owners.iter().rev().find_map(|(_, read)| read.clone());
    let marked = |one: &Described, key: &str| {
        let mut pairs = one.annotations.iter().chain(one.labels.iter());
        pairs
            .find(|(at, _)| at == key)
            .map(|(_, value)| value.clone())
    };
    for one in top.as_deref().into_iter().chain(std::iter::once(object)) {
        if let Some(name) = marked(one, "meta.helm.sh/release-name") {
            let namespace = marked(one, "meta.helm.sh/release-namespace");
            return Some((namespace.or(one.namespace.clone())?, name));
        }
        if marked(one, "app.kubernetes.io/managed-by").as_deref() == Some("Helm")
            && let Some(name) = marked(one, "app.kubernetes.io/instance")
        {
            return Some((one.namespace.clone()?, name));
        }
    }
    None
}

/// The events about the object, once its uid is known.
pub fn events_key(app: &AppState, link: &Link, uid: &str) -> Option<FollowKey> {
    Some(FollowKey {
        context: link.context.clone(),
        kind: served(app, &link.context, "", "Event")?,
        namespace: link.namespace.clone(),
        fields: Some(format!("involvedObject.uid={uid}")),
    })
}

/// Every Service of the object's namespace.
pub fn services_key(app: &AppState, link: &Link) -> Option<FollowKey> {
    Some(FollowKey {
        context: link.context.clone(),
        kind: served(app, &link.context, "", "Service")?,
        namespace: link.namespace.clone(),
        fields: None,
    })
}

/// The Services whose selector picks the object by its labels.
pub fn selecting(app: &AppState, link: &Link, object: &Described) -> Vec<String> {
    let Some(services) =
        services_key(app, link).and_then(|key| app.cluster.store.follows.get(&key))
    else {
        return Vec::new();
    };
    let picks = |one: &&Arc<Described>| {
        !one.selector.is_empty() && one.selector.iter().all(|pair| object.labels.contains(pair))
    };
    services
        .objects
        .iter()
        .filter(picks)
        .map(|one| one.name.clone())
        .collect()
}
