//! The choices of *Attach cluster* and *Detach cluster*, and how a choice reads back.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, session};
use groove_types::Attached;

use super::{Action, Flow};

/// What separates a held context from its namespace in a choice's value.
const BETWEEN: char = '\u{1}';

/// The contexts Groove knows, in Settings' order.
pub(super) fn known(app: &AppState) -> Vec<(String, String)> {
    let contexts = app.config.clusters().iter();
    contexts
        .map(|one| (one.context.clone(), one.context.clone()))
        .collect()
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
