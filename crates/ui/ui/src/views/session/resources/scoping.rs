//! The scope pickers' panel: the contexts held and those to attach, or every `context/namespace`.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster, session};
use groove_types::{Attached, fuzzy};
use groove_ui_kit::widgets::Field;

use super::scope::{ResourcesUi, WHOLE_MARK, contexts, focus, toggle};
use crate::hit::Picks;

/// The panel under a scope picker: its search, and the line the keys stand on.
#[derive(Debug, Clone, PartialEq)]
pub struct Scoping {
    pub which: Picks,
    pub query: Field,
    /// None stands on the first line that acts.
    pub cursor: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeLine {
    /// A section of the contexts.
    Heading(String),
    /// A context the session holds.
    Held(String),
    /// A context Settings knows that the session does not hold.
    Known(String),
    /// One namespace of a context held, or its whole cluster.
    Namespace(Attached),
}

impl ScopeLine {
    pub fn label(&self) -> String {
        match self {
            ScopeLine::Heading(text) | ScopeLine::Held(text) | ScopeLine::Known(text) => {
                text.clone()
            }
            ScopeLine::Namespace(pair) => named(pair),
        }
    }
}

/// `staging/shop`, or `staging/*` for the whole cluster.
fn named(pair: &Attached) -> String {
    let namespace = pair.namespace.as_deref().unwrap_or(WHOLE_MARK);
    format!("{}/{namespace}", pair.context)
}

impl Scoping {
    pub fn new(which: Picks) -> Self {
        Self {
            which,
            query: Field::default(),
            cursor: None,
        }
    }

    /// The namespace panel, or the context one while the session holds no cluster.
    pub fn asked(open: &Open) -> Self {
        match !open.holds_clusters() {
            true => Self::new(Picks::Contexts),
            false => Self::new(Picks::Namespaces),
        }
    }

    /// The panel opened: the namespaces of every whole cluster held listed again.
    pub fn opened(self, open: &Open) -> (Self, Vec<Command>) {
        let list = |context: &str| {
            let context = context.to_string();
            Command::Cluster(cluster::Command::ListNamespaces { context })
        };
        let commands = match self.which {
            Picks::Contexts => Vec::new(),
            _ => wholes(open).into_iter().map(list).collect(),
        };
        (self, commands)
    }
}

/// The contexts the session holds as a whole cluster.
fn wholes(open: &Open) -> Vec<&str> {
    let whole = |context: &&str| {
        let held = Attached {
            context: context.to_string(),
            namespace: None,
        };
        open.clusters.contains(&held)
    };
    contexts(open).into_iter().filter(whole).collect()
}

/// What the panel lists under its search.
pub fn lines(app: &AppState, open: &Open, scoping: &Scoping) -> Vec<ScopeLine> {
    let query = scoping.query.text();
    match scoping.which {
        Picks::Contexts => context_lines(app, open, query),
        _ => namespace_lines(app, open, query),
    }
}

fn context_lines(app: &AppState, open: &Open, query: &str) -> Vec<ScopeLine> {
    let hit = |label: &str| query.is_empty() || fuzzy(label, query);
    let held = contexts(open);
    let mut out = Vec::new();
    let mine: Vec<ScopeLine> = held
        .iter()
        .filter(|one| hit(one))
        .map(|one| ScopeLine::Held(one.to_string()))
        .collect();
    if !mine.is_empty() {
        out.push(ScopeLine::Heading("in this session".into()));
        out.extend(mine);
    }
    let known: Vec<ScopeLine> = app
        .config
        .clusters()
        .iter()
        .map(|one| one.context.as_str())
        .filter(|one| !held.contains(one) && hit(one))
        .map(|one| ScopeLine::Known(one.to_string()))
        .collect();
    if !known.is_empty() {
        out.push(ScopeLine::Heading("available".into()));
        out.extend(known);
    }
    out
}

/// Every pair the session lets the list read, the best match of `query` first.
fn namespace_lines(app: &AppState, open: &Open, query: &str) -> Vec<ScopeLine> {
    let whole = wholes(open);
    let mut pairs: Vec<Attached> = Vec::new();
    for context in contexts(open) {
        let pair = |namespace: Option<&str>| Attached {
            context: context.to_string(),
            namespace: namespace.map(String::from),
        };
        if !whole.contains(&context) {
            let held = open.clusters.iter().filter(|one| one.context == context);
            pairs.extend(held.cloned());
            continue;
        }
        pairs.push(pair(None));
        let listed = app.cluster.store.namespaces(context).unwrap_or_default();
        pairs.extend(listed.iter().map(|one| pair(Some(one))));
    }
    let ranked = groove_types::ranked(pairs, named, query);
    ranked.into_iter().map(ScopeLine::Namespace).collect()
}

/// How a line is chosen: a click or space, Enter, or its ×.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Toggle,
    Alone,
    Detach,
}

/// The line chosen: a context shown, hidden, alone, detached or attached; a namespace shown or alone.
pub fn choose(
    line: &ScopeLine,
    how: How,
    (open, held, scoping): (&Open, &mut ResourcesUi, &mut Scoping),
) -> Vec<Command> {
    held.scroll = 0.0;
    match line {
        ScopeLine::Heading(_) => Vec::new(),
        ScopeLine::Held(context) => kept(context, how, open, held),
        ScopeLine::Known(context) => {
            if how == How::Alone {
                held.hidden_contexts = contexts(open).into_iter().map(String::from).collect();
            }
            held.picked.retain(|one| one.context != *context);
            (scoping.query, scoping.cursor) = (Field::default(), None);
            let attach = session::Command::AttachCluster {
                session: open.session.id.clone(),
                attached: Attached {
                    context: context.clone(),
                    namespace: None,
                },
            };
            vec![Command::Session(attach)]
        }
        ScopeLine::Namespace(pair) => {
            match how {
                How::Toggle => toggle(held, open, pair),
                How::Alone => focus(held, open, pair),
                How::Detach => {}
            }
            Vec::new()
        }
    }
}

/// A held context shown or hidden, shown alone, or detached.
fn kept(context: &str, how: How, open: &Open, held: &mut ResourcesUi) -> Vec<Command> {
    match how {
        How::Toggle => {
            groove_types::toggle(&mut held.hidden_contexts, context.to_string());
            Vec::new()
        }
        How::Alone => {
            held.hidden_contexts = contexts(open)
                .into_iter()
                .filter(|one| *one != context)
                .map(String::from)
                .collect();
            Vec::new()
        }
        How::Detach => {
            held.hidden_contexts.remove(context);
            held.picked.retain(|one| one.context != context);
            let pairs = open.clusters.iter().filter(|one| one.context == context);
            let detach = |attached: &Attached| {
                Command::Session(session::Command::DetachCluster {
                    session: open.session.id.clone(),
                    attached: attached.clone(),
                })
            };
            pairs.map(detach).collect()
        }
    }
}
