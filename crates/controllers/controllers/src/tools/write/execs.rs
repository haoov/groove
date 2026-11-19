//! The agent's commands in a pod: queued with the command shown, run once approved.

use std::time::Duration;

use groove_agent_service::tools::Arguments;
use groove_cluster_service::{Exec, LONGEST, RUNS};
use groove_types::{KubeKind, Target, TimelineKind};
use serde_json::json;

use super::Write;
use crate::cluster::writes::allowed;
use crate::{AppState, Continuation, Services, Spawner};

/// A command checked against the session, then waiting with what it runs, whatever auto-approve says.
pub(super) fn asked(state: &mut AppState, spawner: &dyn Spawner, mut write: Write) {
    let exec = match named(&write) {
        Ok(exec) => exec,
        Err(why) => return write.reply.failed(why),
    };
    if let Err(why) = runnable(state, &write, &exec) {
        return write.reply.failed(why);
    }
    if state.agent.agent(&write.session).is_none() {
        let gone = "the session closed before kube_exec ran";
        return write.reply.failed(gone);
    }
    let container = exec.container.as_deref().unwrap_or("first");
    let (context, namespace) = (&exec.context, &exec.namespace);
    let path = format!("{context}/{namespace}/{}/{container}.sh", exec.pod);
    let shown = exec.shown();
    write.arguments["target"] = pod(&exec).label().into();
    write.arguments["change"] = json!([path, "", format!("{shown}\n")]);
    write.arguments["shown"] = shown.into();
    super::queued(state, write);
    crate::workspace::review::show(state, spawner);
}

/// The command the user approved, run where it was shown.
pub(super) fn run(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let exec = match named(&write) {
        Ok(exec) => exec,
        Err(why) => return write.reply.failed(why),
    };
    if let Err(why) = runnable(state, &write, &exec) {
        return write.reply.failed(why);
    }
    let subject = format!("{} · exec {}", pod(&exec).label(), exec.shown());
    spawner.spawn(Box::pin(async move {
        let ran = groove_cluster_service::exec(&exec).await;
        Box::new(
            move |_: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                let said = match &ran {
                    Ok(answer) if answer.failed => format!("{subject} · failed"),
                    Ok(_) => subject,
                    Err(e) => format!("{subject} · {}", e.message),
                };
                let kind = TimelineKind::ClusterWrite;
                crate::timeline::said(services, spawner, &write.session, kind, said);
                match ran {
                    Ok(answer) if answer.failed => write.reply.failed(answer.text),
                    Ok(answer) => write.reply.said(answer.text),
                    Err(e) => write.reply.failed(e.message),
                }
            },
        ) as Continuation
    }));
}

/// Refused on a context set read-only, or a namespace the session does not hold.
fn runnable(state: &AppState, write: &Write, exec: &Exec) -> Result<(), String> {
    let allowed = allowed(state, &write.session, &exec.context).map_err(|e| e.message)?;
    let pod = pod(exec);
    groove_cluster_service::writable(&pod, allowed.read_only, &allowed.held).map_err(|e| e.message)
}

fn named(write: &Write) -> Result<Exec, String> {
    let needs = |name: &str| {
        let named = write.text(name).map(str::to_string);
        named.ok_or(format!("kube_exec needs the {name}"))
    };
    let command = write.arguments.texts("command").unwrap_or_default();
    let seconds = write.number("timeout").map(|one| one.max(1) as u64);
    let timeout = seconds.map_or(RUNS, |one| Duration::from_secs(one).min(LONGEST));
    let exec = Exec {
        context: needs("context")?,
        namespace: needs("namespace")?,
        pod: needs("pod")?,
        container: write.text("container").map(str::to_string),
        command,
        timeout,
    };
    exec.argv().map_err(|e| e.message)?;
    Ok(exec)
}

/// The pod a command runs in, as the write checks name an object.
fn pod(exec: &Exec) -> Target {
    let kind = KubeKind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
        short_names: Vec::new(),
    };
    Target {
        context: exec.context.clone(),
        kind,
        namespace: Some(exec.namespace.clone()),
        name: exec.pod.clone(),
        subresource: None,
    }
}
