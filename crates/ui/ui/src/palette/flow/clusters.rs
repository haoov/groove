//! The choices of *Attach cluster* and *Detach cluster*, and how a choice reads back.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, session};
use groove_types::Attached;

use super::{Action, Flow, Prompt};

/// What separates a held context from its namespace in a choice's value.
const BETWEEN: char = '\u{1}';

/// The contexts Groove knows, in Settings' order.
pub(super) fn known(app: &AppState) -> Vec<(String, String)> {
    let contexts = app.config.clusters().iter();
    contexts
        .map(|one| (one.context.clone(), one.context.clone()))
        .collect()
}

/// The context's namespaces as last listed, behind `*` for the whole cluster; any name typed too.
pub(super) fn namespaces(app: &AppState, context: &str) -> Prompt {
    let listed = app.cluster.store.namespaces(context).unwrap_or_default();
    let whole = std::iter::once(("* the whole cluster".to_string(), String::new()));
    let named = listed.iter().map(|one| (one.clone(), one.clone()));
    Prompt {
        label: "namespace, empty for the whole cluster",
        options: whole.chain(named).collect(),
        free: true,
        allow_empty: true,
    }
}

/// Lists the context's namespaces again.
pub(super) fn refresh(context: &str) -> groove_controllers::Command {
    let list = groove_controllers::cluster::Command::ListNamespaces {
        context: context.to_string(),
    };
    groove_controllers::Command::Cluster(list)
}

/// What the session holds, one choice a context and namespace.
pub(super) fn held(open: &Open) -> Vec<(String, String)> {
    open.clusters
        .iter()
        .map(|one| {
            let namespace = one.namespace.as_deref().unwrap_or_default();
            let shown = one.namespace.as_deref().unwrap_or("whole cluster");
            let value = format!("{}{BETWEEN}{namespace}", one.context);
            (format!("{} · {shown}", one.context), value)
        })
        .collect()
}

/// The attach or the detach, once every answer is in.
pub(super) fn command(flow: &Flow) -> Option<session::Command> {
    let (session, answer) = (flow.session.clone(), |at: usize| flow.answers.get(at));
    match flow.action {
        Action::AttachCluster => Some(session::Command::AttachCluster {
            session,
            attached: Attached {
                context: answer(0)?.clone(),
                namespace: answer(1).filter(|one| !one.is_empty()).cloned(),
            },
        }),
        Action::DetachCluster => Some(session::Command::DetachCluster {
            session,
            attached: of(answer(0)?),
        }),
        _ => None,
    }
}

fn of(value: &str) -> Attached {
    let (context, namespace) = value.split_once(BETWEEN).unwrap_or((value, ""));
    Attached {
        context: context.to_string(),
        namespace: (!namespace.is_empty()).then(|| namespace.to_string()),
    }
}

/// Every namespace the session holds, by its label, behind all of them as `*`.
pub(super) fn scopes(open: &Open) -> Vec<(String, String)> {
    let all = std::iter::once(("all namespaces".to_string(), "*".to_string()));
    let pairs = open.clusters.iter().map(|one| {
        let label = crate::views::session::resources::Pick::Pair(one.clone()).label();
        (label.clone(), label)
    });
    all.chain(pairs).collect()
}
