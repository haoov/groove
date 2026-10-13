//! The objects open in tabs of their own.

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

    /// The pod's context, namespace and name, which its usage is kept under.
    pub fn pod(&self) -> groove_controllers::cluster_service::Named {
        let namespace = self.namespace.clone().unwrap_or_default();
        (self.context.clone(), namespace, self.name.clone())
    }

    /// The object as last read, once its watcher has listed it.
    pub fn read<'a>(&self, app: &'a AppState) -> Option<&'a Arc<Described>> {
        app.cluster.store.object(&self.key())
    }
}

/// What a ctrl+click on a value opens: an object's tab, a repo in the session, or one of its commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opens {
    Object(Link),
    Repo {
        repo: String,
        target: String,
    },
    Revision {
        repo: String,
        target: String,
        sha: String,
    },
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

/// One object's tab: which view, what is folded, how far down.
#[derive(Debug, Clone, PartialEq)]
pub struct Opened {
    pub link: Link,
    pub view: super::View,
    pub logs: super::LogsUi,
    pub argo: super::ArgoUi,
    /// The column the rows of each group sort by, and whether descending.
    pub sort: Option<(String, bool)>,
    /// The groups and rows folded against how they start.
    pub toggled: std::collections::BTreeMap<String, bool>,
    /// The sections folded shut; the annotations, the relations and the events until opened.
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
            argo: super::ArgoUi::default(),
            sort: None,
            toggled: std::collections::BTreeMap::new(),
            shut: [Section::Annotations, Section::Relations, Section::Events].into(),
            scroll: 0.0,
            across: 0.0,
            copied: None,
        }
    }

    /// `key` turned the other way from how it was drawn, `shut` or open.
    pub fn fold(&mut self, key: &str, shut: bool) {
        self.toggled.insert(key.to_string(), !shut);
    }

    /// Whether `key` stands folded: as the user left it, else as it starts.
    pub fn folded(&self, key: &str, folds: bool) -> bool {
        self.toggled.get(key).copied().unwrap_or(folds)
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
        (self.showing, self.typing) = (Some(at), false);
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

    /// How far the YAML shown is scrolled sideways, or the list.
    pub fn across(&self) -> f32 {
        self.tab().map_or(self.across, |one| one.across)
    }

    pub fn across_mut(&mut self) -> &mut f32 {
        let at = self.showing.filter(|at| *at < self.opened.len());
        match at {
            Some(at) => &mut self.opened[at].across,
            None => &mut self.across,
        }
    }
}
