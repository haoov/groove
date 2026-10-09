//! What the Resources tab reads: the contexts and namespaces picked, the kind, the search.

use std::collections::BTreeSet;

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster};
use groove_types::{Attached, KubeKind, WatchKey};
use groove_ui_kit::widgets::Field;

/// The one reader the tab holds its watchers under.
pub const READER: &str = "resources";

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResourcesUi {
    /// What the pickers leave out; nothing left out is everything the session holds.
    pub hidden_contexts: BTreeSet<String>,
    pub hidden: BTreeSet<Attached>,
    /// The kind picked in the sidebar; none is the pods.
    pub kind: Option<KubeKind>,
    /// The list's find bar: words narrow by name, a word with `=` is a label selector.
    pub search: Field,
    /// The find bar stands over the list, and has the keys.
    pub finding: bool,
    pub typing: bool,
    /// The sidebar's search, which narrows the kinds, and whether it has the keys.
    pub filter: Field,
    pub filtering: bool,
    /// The kind the arrows stand on, by its place among the kinds shown.
    pub cursor: Option<usize>,
    /// The sidebar's headings folded shut.
    pub folded: BTreeSet<groove_types::KindHeading>,
    /// The column the rows are ordered by, and whether downwards.
    pub sort: Option<(String, bool)>,
    /// The widths dragged, by kind, then by column.
    pub widths: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f32>>,
    /// The list's order as the last frame built it.
    pub kept: super::list::Kept,
    /// The objects open in tabs beside the list, and the one shown; none is the list.
    pub opened: Vec<super::opened::Opened>,
    pub showing: Option<usize>,
    pub scroll: f32,
}

/// A column's edge held: its kind, its label, where the press was, and its width then.
#[derive(Debug, Clone, PartialEq)]
pub struct Dragged {
    pub kind: String,
    pub label: String,
    pub from: f32,
    pub width: f32,
}

/// One row of a scope picker: a context, or one context and namespace pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    Context(String),
    Pair(Attached),
}

impl Pick {
    pub fn label(&self) -> String {
        match self {
            Pick::Context(context) => context.clone(),
            Pick::Pair(pair) => {
                let namespace = pair.namespace.as_deref().unwrap_or("*");
                format!("{} · {namespace}", pair.context)
            }
        }
    }

    pub fn shown(&self, ui: &ResourcesUi) -> bool {
        match self {
            Pick::Context(context) => !ui.hidden_contexts.contains(context),
            Pick::Pair(pair) => !ui.hidden.contains(pair),
        }
    }

    pub fn toggle(&self, ui: &mut ResourcesUi) {
        match self {
            Pick::Context(context) => flip(&mut ui.hidden_contexts, context.clone()),
            Pick::Pair(pair) => flip(&mut ui.hidden, pair.clone()),
        }
        ui.scroll = 0.0;
    }
}

/// The scope narrowed to one pair by its label, or opened to every pair the session holds by `*`.
pub fn focus(held: &mut ResourcesUi, open: &Open, scope: &str) {
    held.hidden_contexts.clear();
    held.hidden = open
        .clusters
        .iter()
        .filter(|one| scope != "*" && Pick::Pair((*one).clone()).label() != scope)
        .cloned()
        .collect();
    held.scroll = 0.0;
}

/// The contexts the session holds, in the order first attached.
pub fn contexts(open: &Open) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for one in &open.clusters {
        if !out.contains(&one.context.as_str()) {
            out.push(&one.context);
        }
    }
    out
}

/// The pairs whose context is picked: what the namespace picker offers.
pub fn offered<'a>(open: &'a Open, ui: &ResourcesUi) -> impl Iterator<Item = &'a Attached> {
    let hidden = ui.hidden_contexts.clone();
    open.clusters
        .iter()
        .filter(move |one| !hidden.contains(&one.context))
}

/// The pairs the list reads.
pub fn held<'a>(open: &'a Open, ui: &'a ResourcesUi) -> impl Iterator<Item = &'a Attached> {
    offered(open, ui).filter(|one| !ui.hidden.contains(one))
}

/// The kind picked, or the pods of the first context whose kinds are known.
pub fn kind(app: &AppState, open: &Open, ui: &ResourcesUi) -> Option<KubeKind> {
    if let Some(kind) = &ui.kind {
        return Some(kind.clone());
    }
    let known = contexts(open)
        .into_iter()
        .find_map(|one| app.cluster.store.kinds(one))?;
    let pods = |one: &&KubeKind| one.group.is_empty() && one.kind == "Pod";
    known.iter().find(pods).cloned()
}

/// The search's label selector, its `=` words joined; and its name words.
pub fn search(ui: &ResourcesUi) -> (Option<String>, Vec<&str>) {
    let (labels, names): (Vec<&str>, Vec<&str>) = ui
        .search
        .text()
        .split_whitespace()
        .partition(|word| word.contains('='));
    let selector = (!labels.is_empty()).then(|| labels.join(","));
    (selector, names)
}

/// One key a pair, or one a context for a kind that is not namespaced; each context's own kind.
pub fn keys(app: &AppState, open: &Open, ui: &ResourcesUi) -> Vec<WatchKey> {
    let Some(picked) = kind(app, open, ui) else {
        return Vec::new();
    };
    let (selector, _) = search(ui);
    let mut out: Vec<WatchKey> = Vec::new();
    for pair in held(open, ui) {
        let Some(kinds) = app.cluster.store.kinds(&pair.context) else {
            continue;
        };
        let same = |one: &&KubeKind| one.group == picked.group && one.kind == picked.kind;
        let Some(kind) = kinds.iter().find(same).filter(|one| one.watchable) else {
            continue;
        };
        let key = WatchKey {
            context: pair.context.clone(),
            namespace: pair.namespace.clone().filter(|_| kind.namespaced),
            kind: kind.clone(),
            selector: selector.clone(),
        };
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// What the tab needs each frame, `up` while it shows: its contexts' kinds; a watcher a key while the list shows.
pub fn wants(app: &AppState, up: bool, ui: &ResourcesUi) -> Vec<Command> {
    let open = app.session.selected().filter(|_| up);
    let mut out = Vec::new();
    let wanted = match open {
        Some(open) => {
            out.extend(discoveries(app, open));
            match ui.tab() {
                Some(_) => Vec::new(),
                None => keys(app, open, ui),
            }
        }
        None => Vec::new(),
    };
    out.extend(leases(app, wanted));
    out
}

/// A discovery for each context in scope whose kinds are neither known nor asked for.
fn discoveries(app: &AppState, open: &Open) -> Vec<Command> {
    let store = &app.cluster.store;
    let unknown = |one: &&str| store.kinds(one).is_none() && !store.discovering(one);
    let discover = |one: &str| cluster::Command::Discover {
        context: one.to_string(),
        again: false,
    };
    contexts(open)
        .into_iter()
        .filter(unknown)
        .map(|one| Command::Cluster(discover(one)))
        .collect()
}

/// Nothing while the keys held are the keys wanted; else let go of them all and read the new.
fn leases(app: &AppState, wanted: Vec<WatchKey>) -> Vec<Command> {
    let held: BTreeSet<&WatchKey> = app.cluster.store.read_by(READER).collect();
    if held == wanted.iter().collect() {
        return Vec::new();
    }
    let release = cluster::Command::Release {
        reader: READER.into(),
    };
    let watch = |key| {
        Command::Cluster(cluster::Command::Watch {
            reader: READER.into(),
            key,
        })
    };
    std::iter::once(Command::Cluster(release))
        .chain(wanted.into_iter().map(watch))
        .collect()
}

fn flip<T: Ord>(set: &mut BTreeSet<T>, one: T) {
    if !set.remove(&one) {
        set.insert(one);
    }
}
