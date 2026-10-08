//! The scope pickers' panel: what the session holds, shown or not, then what it could attach.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, cluster, session};
use groove_types::{Attached, fuzzy};
use groove_ui_kit::widgets::Field;

use super::scope::{Pick, ResourcesUi, contexts, focus};
use crate::hit::Picks;

pub const WHOLE: &str = "* whole cluster";

/// The panel under a scope picker: its search, the line the keys stand on, a context being attached.
#[derive(Debug, Clone, PartialEq)]
pub struct Scoping {
    pub which: Picks,
    pub query: Field,
    pub cursor: Option<usize>,
    /// A context picked among the available ones, until one of its namespaces is.
    pub adding: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeLine {
    /// A context's name over its namespaces, or a section of the contexts.
    Heading(String),
    Held(Pick),
    /// A context Settings knows that the session does not hold.
    Known(String),
    /// A namespace to attach; `typed` when only the search names it.
    Free {
        attached: Attached,
        typed: bool,
    },
}

impl ScopeLine {
    pub fn label(&self) -> String {
        match self {
            ScopeLine::Heading(text) | ScopeLine::Known(text) => text.clone(),
            ScopeLine::Held(Pick::Context(context)) => context.clone(),
            ScopeLine::Held(Pick::Pair(pair)) => named(pair).to_string(),
            ScopeLine::Free { attached, typed } => match typed {
                true => format!("attach {}", named(attached)),
                false => named(attached).to_string(),
            },
        }
    }
}

fn named(pair: &Attached) -> &str {
    pair.namespace.as_deref().unwrap_or(WHOLE)
}

impl Scoping {
    pub fn new(which: Picks) -> Self {
        Self {
            which,
            query: Field::default(),
            cursor: None,
            adding: None,
        }
    }

    /// The namespace panel, or the context one while the session holds no cluster.
    pub fn asked(open: &Open) -> Self {
        match open.clusters.is_empty() {
            true => Self::new(Picks::Contexts),
            false => Self::new(Picks::Namespaces),
        }
    }

    /// The panel opened: the namespaces of its contexts listed again.
    pub fn opened(self, open: &Open, held: &ResourcesUi) -> (Self, Vec<Command>) {
        let list = |context: String| Command::Cluster(cluster::Command::ListNamespaces { context });
        let commands = match self.which {
            Picks::Contexts => Vec::new(),
            _ => in_panel(open, held, &self).into_iter().map(list).collect(),
        };
        (self, commands)
    }
}

/// The contexts whose namespaces the panel lists: those picked, then one being attached.
fn in_panel(open: &Open, held: &ResourcesUi, scoping: &Scoping) -> Vec<String> {
    let picked = contexts(open)
        .into_iter()
        .filter(|one| !held.hidden_contexts.contains(*one));
    let mut out: Vec<String> = picked.map(String::from).collect();
    if let Some(adding) = scoping.adding.clone().filter(|one| !out.contains(one)) {
        out.push(adding);
    }
    out
}

/// What the panel lists under its search.
pub fn lines(app: &AppState, open: &Open, held: &ResourcesUi, scoping: &Scoping) -> Vec<ScopeLine> {
    let query = scoping.query.text();
    let hit = |label: &str| query.is_empty() || fuzzy(label, query);
    match scoping.which {
        Picks::Contexts => context_lines(app, open, &hit),
        _ => in_panel(open, held, scoping)
            .into_iter()
            .flat_map(|context| namespace_lines(app, open, &context, (query, &hit)))
            .collect(),
    }
}

fn context_lines(app: &AppState, open: &Open, hit: &dyn Fn(&str) -> bool) -> Vec<ScopeLine> {
    let held = contexts(open);
    let mut out = Vec::new();
    let mine: Vec<ScopeLine> = held
        .iter()
        .filter(|one| hit(one))
        .map(|one| ScopeLine::Held(Pick::Context(one.to_string())))
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

/// One context: its name, the pairs held, then what it could attach.
fn namespace_lines(
    app: &AppState,
    open: &Open,
    context: &str,
    (query, hit): (&str, &dyn Fn(&str) -> bool),
) -> Vec<ScopeLine> {
    let pairs: Vec<&Attached> = open
        .clusters
        .iter()
        .filter(|one| one.context == context)
        .collect();
    let mut out: Vec<ScopeLine> = pairs
        .iter()
        .filter(|one| hit(named(one)))
        .map(|one| ScopeLine::Held(Pick::Pair((*one).clone())))
        .collect();
    let whole = pairs.iter().any(|one| one.namespace.is_none());
    let free = |namespace: Option<&str>, typed| ScopeLine::Free {
        attached: Attached {
            context: context.to_string(),
            namespace: namespace.map(String::from),
        },
        typed,
    };
    let taken = |name: &str| {
        pairs
            .iter()
            .any(|one| one.namespace.as_deref() == Some(name))
    };
    if !whole && hit(WHOLE) {
        out.push(free(None, false));
    }
    let listed = app.cluster.store.namespaces(context).unwrap_or_default();
    let open_ones = listed.iter().filter(|one| !taken(one) && hit(one));
    out.extend(open_ones.map(|one| free(Some(one), false)));
    let named_already = listed.iter().any(|one| one == query) || taken(query);
    if valid(query) && !named_already {
        out.push(free(Some(query), true));
    }
    if out.is_empty() {
        return out;
    }
    std::iter::once(ScopeLine::Heading(context.to_string()))
        .chain(out)
        .collect()
}

/// A name Kubernetes takes for a namespace.
fn valid(name: &str) -> bool {
    let fits = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    !name.is_empty() && name.len() <= 63 && name.chars().all(fits)
}

/// How a line is chosen: a click or space, Enter, or its ×.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    Toggle,
    Alone,
    Detach,
}

/// The line chosen: a held one shown, hidden, alone or detached; any other attached or opened.
pub fn choose(
    line: &ScopeLine,
    how: How,
    (open, held, scoping): (&Open, &mut ResourcesUi, &mut Scoping),
) -> Vec<Command> {
    held.scroll = 0.0;
    match line {
        ScopeLine::Heading(_) => Vec::new(),
        ScopeLine::Held(pick) => kept(pick, how, open, held),
        ScopeLine::Known(context) => {
            *scoping = Scoping {
                adding: Some(context.clone()),
                ..Scoping::new(Picks::Namespaces)
            };
            let list = cluster::Command::ListNamespaces {
                context: context.clone(),
            };
            vec![Command::Cluster(list)]
        }
        ScopeLine::Free { attached, .. } => {
            held.hidden_contexts.remove(&attached.context);
            if scoping.adding.as_ref() == Some(&attached.context) {
                scoping.adding = None;
            }
            (scoping.query, scoping.cursor) = (Field::default(), None);
            let attach = session::Command::AttachCluster {
                session: open.session.id.clone(),
                attached: attached.clone(),
            };
            vec![Command::Session(attach)]
        }
    }
}

/// A held line shown or hidden, shown alone, or detached.
fn kept(pick: &Pick, how: How, open: &Open, held: &mut ResourcesUi) -> Vec<Command> {
    let detach = |attached: Attached| {
        Command::Session(session::Command::DetachCluster {
            session: open.session.id.clone(),
            attached,
        })
    };
    match (pick, how) {
        (pick, How::Toggle) => {
            pick.toggle(held);
            Vec::new()
        }
        (Pick::Context(context), How::Alone) => {
            held.hidden_contexts = contexts(open)
                .into_iter()
                .filter(|one| one != context)
                .map(String::from)
                .collect();
            Vec::new()
        }
        (pick @ Pick::Pair(_), How::Alone) => {
            focus(held, open, &pick.label());
            Vec::new()
        }
        (Pick::Context(context), How::Detach) => {
            held.hidden_contexts.remove(context);
            let pairs = open.clusters.iter().filter(|one| &one.context == context);
            pairs.cloned().map(detach).collect()
        }
        (Pick::Pair(pair), How::Detach) => {
            held.hidden.remove(pair);
            vec![detach(pair.clone())]
        }
    }
}
