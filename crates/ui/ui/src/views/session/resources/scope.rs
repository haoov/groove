//! What the Resources tab reads: the contexts and namespaces picked, the kind, the search.

use std::collections::BTreeSet;

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster};
use groove_types::{Attached, KubeKind, Timestamp, WatchKey};
use groove_ui_kit::widgets::Field;

/// The whole cluster, where a namespace would stand in a short label.
pub const WHOLE_MARK: &str = "*";

/// The whole cluster, as a name under its context.
pub const WHOLE_NAME: &str = "whole cluster";

/// The one reader the tab holds its watchers under.
pub const READER: &str = "resources";

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResourcesUi {
    /// What the pickers leave out; nothing left out is everything the session holds.
    pub hidden_contexts: BTreeSet<String>,
    /// The namespaces picked; a context with none picked shows all it holds.
    pub picked: BTreeSet<Attached>,
    /// The kind picked in the sidebar; none is the pods.
    pub kind: Option<KubeKind>,
    /// The list's find bar: words narrow by name, a word with `=` is a label selector.
    pub search: Field,
    /// The find bar stands over the list, and has the keys.
    pub finding: bool,
    pub typing: bool,
    /// The kind the arrows stand on, by its place among the kinds shown.
    pub cursor: Option<usize>,
    /// The sidebar's headings folded shut.
    pub folded: BTreeSet<groove_types::KindHeading>,
    /// The column the rows are ordered by, and whether downwards.
    pub sort: Option<(String, bool)>,
    /// The widths dragged in the list, by kind, then by column.
    pub widths: std::collections::BTreeMap<String, std::collections::BTreeMap<String, f32>>,
    /// The widths dragged in an object's tab, by column.
    pub tab_widths: std::collections::BTreeMap<String, f32>,
    /// The list's order as the last frame built it.
    pub kept: super::list::Kept,
    /// The objects open in tabs beside the list, and the one shown; none is the list.
    pub opened: Vec<super::opened::Opened>,
    pub showing: Option<usize>,
    pub scroll: f32,
    /// How far the list is scrolled sideways.
    pub across: f32,
    /// How far the kinds sidebar is scrolled.
    pub kinds: f32,
}

/// A column's edge held: its kind, none in an object's tab; its label, where the press was, and its width then.
#[derive(Debug, Clone, PartialEq)]
pub struct Dragged {
    pub kind: Option<String>,
    pub label: String,
    pub from: f32,
    pub width: f32,
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

/// The contexts the session holds that the picker leaves in.
pub fn shown_contexts<'a>(open: &'a Open, ui: &ResourcesUi) -> Vec<&'a str> {
    let mut out = contexts(open);
    out.retain(|one| !ui.hidden_contexts.contains(*one));
    out
}

/// The pairs the session holds whose context is picked.
pub fn offered<'a>(open: &'a Open, ui: &ResourcesUi) -> impl Iterator<Item = &'a Attached> {
    let hidden = ui.hidden_contexts.clone();
    open.clusters
        .iter()
        .filter(move |one| !hidden.contains(&one.context))
}

/// Whether the session lets the list read `pair`: held as it is, or within a whole cluster held.
pub fn holds(open: &Open, pair: &Attached) -> bool {
    open.clusters
        .iter()
        .any(|one| one.context == pair.context && (one.namespace.is_none() || one == pair))
}

/// The pairs one context shows: those picked, else every one the session holds there.
fn showing(open: &Open, ui: &ResourcesUi, context: &str) -> Vec<Attached> {
    let picked: Vec<Attached> = ui
        .picked
        .iter()
        .filter(|one| one.context == context && holds(open, one))
        .cloned()
        .collect();
    match picked.is_empty() {
        true => open
            .clusters
            .iter()
            .filter(|one| one.context == context)
            .cloned()
            .collect(),
        false => picked,
    }
}

/// The pairs the list reads.
pub fn held(open: &Open, ui: &ResourcesUi) -> Vec<Attached> {
    shown_contexts(open, ui)
        .into_iter()
        .flat_map(|context| showing(open, ui, context))
        .collect()
}

/// Whether the list reads `pair` as it stands.
pub fn shows(open: &Open, ui: &ResourcesUi, pair: &Attached) -> bool {
    !ui.hidden_contexts.contains(&pair.context) && showing(open, ui, &pair.context).contains(pair)
}

/// The list on `pair` alone.
pub fn focus(held: &mut ResourcesUi, open: &Open, pair: &Attached) {
    held.hidden_contexts = contexts(open)
        .into_iter()
        .filter(|one| *one != pair.context)
        .map(String::from)
        .collect();
    held.picked.retain(|one| one.context != pair.context);
    held.picked.insert(pair.clone());
    held.scroll = 0.0;
}

/// `pair` shown or left out; a namespace picked under a whole cluster shown narrows it.
pub fn toggle(held: &mut ResourcesUi, open: &Open, pair: &Attached) {
    let context = pair.context.clone();
    let mut now = match held.hidden_contexts.remove(&context) {
        true => Vec::new(),
        false => showing(open, held, &context),
    };
    let whole = now.iter().any(|one| one.namespace.is_none());
    match now.iter().position(|one| one == pair) {
        Some(at) => _ = now.remove(at),
        None if whole || pair.namespace.is_none() => now = vec![pair.clone()],
        None => now.push(pair.clone()),
    }
    held.picked.retain(|one| one.context != context);
    if now.is_empty() {
        held.hidden_contexts.insert(context);
    }
    held.picked.extend(now);
    held.scroll = 0.0;
}

/// The kind picked, or the pods of the first context whose kinds are known.
pub fn kind(app: &AppState, open: &Open, ui: &ResourcesUi) -> Option<KubeKind> {
    if let Some(kind) = &ui.kind {
        return Some(kind.clone());
    }
    let store = &app.cluster.store;
    let known = contexts(open)
        .into_iter()
        .find(|one| store.kinds(one).is_some())?;
    store.served(known, "", "Pod")
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
/// While the search is typed, the selector stays the one the watchers hold.
pub fn keys(app: &AppState, open: &Open, ui: &ResourcesUi) -> Vec<WatchKey> {
    let Some(picked) = kind(app, open, ui) else {
        return Vec::new();
    };
    let selector = match ui.typing {
        true => app
            .cluster
            .store
            .read_by(READER)
            .find_map(|one| one.selector.clone()),
        false => search(ui).0,
    };
    let mut out: Vec<WatchKey> = Vec::new();
    for pair in held(open, ui)
        .iter()
        .filter(|one| app.config.cluster(&one.context).is_some())
    {
        let served = app
            .cluster
            .store
            .served(&pair.context, &picked.group, &picked.kind);
        let Some(kind) = served.filter(|one| one.watchable) else {
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
pub fn wants(app: &AppState, up: bool, ui: &ResourcesUi, now: Timestamp) -> Vec<Command> {
    let open = app.session.selected().filter(|_| up);
    let mut out = Vec::new();
    let wanted = match open {
        Some(open) => {
            out.extend(discoveries(app, open, now));
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

/// A discovery for each context in scope, still added to Groove, whose kinds are unknown or older than `FRESH`.
fn discoveries(app: &AppState, open: &Open, now: Timestamp) -> Vec<Command> {
    let store = &app.cluster.store;
    let fresh = groove_controllers::cluster_service::FRESH.as_secs() as i64;
    let asked = |one: &str| {
        let added = app.config.cluster(one).is_some();
        if !added || store.discovering(one) {
            return None;
        }
        let again = match store.discovered(one) {
            None => false,
            Some(at) if now.seconds() - at.seconds() >= fresh => true,
            Some(_) => return None,
        };
        let context = one.to_string();
        Some(Command::Cluster(cluster::Command::Discover {
            context,
            again,
        }))
    };
    contexts(open).into_iter().filter_map(asked).collect()
}

/// The watchers held while they are the ones wanted; else the new ones.
fn leases(app: &AppState, wanted: Vec<WatchKey>) -> Vec<Command> {
    let held: BTreeSet<&WatchKey> = app.cluster.store.read_by(READER).collect();
    let same = held == wanted.iter().collect();
    let watch = |key| {
        Command::Cluster(cluster::Command::Watch {
            reader: READER.into(),
            key,
        })
    };
    super::leases(READER, same, wanted.into_iter().map(watch))
}
