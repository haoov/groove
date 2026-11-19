//! A user's shell in a pod: `kubectl exec` on a terminal of the session's drawer.

use groove_cluster_service::{Exec, RUNS, SHELL, writable};
use groove_shell_service::kubectl;
use groove_types::{SessionId, Target};

use super::Made;
use crate::cluster::writes::allowed;
use crate::{AppState, Services, Spawner};

/// The size a pod's terminal starts at, before the drawer fits it.
const SIZE: (u16, u16) = (80, 24);

/// A tab running a shell in the pod's container; a read-only context or an unheld namespace refuses it.
pub(super) fn shell(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (session, pod, container): (SessionId, Target, Option<String>),
) {
    let checked = allowed(state, &session, &pod.context)
        .and_then(|allowed| writable(&pod, allowed.read_only, &allowed.held));
    if let Err(e) = checked {
        return state.failed(e);
    }
    let exec = Exec {
        context: pod.context.clone(),
        namespace: pod.namespace.clone().unwrap_or_default(),
        pod: pod.name.clone(),
        container: container.clone(),
        command: SHELL.map(str::to_string).to_vec(),
        timeout: RUNS,
    };
    let argv = match exec.terminal() {
        Ok(argv) => argv,
        Err(e) => return state.failed(e),
    };
    let home = state.env.home.clone();
    let made: Made = Box::new(move |cwd| kubectl(argv, (cwd, &home), SIZE));
    let id = super::open(state, services, spawner, (session.clone(), false), made);
    let name = match container {
        Some(container) => format!("{} · {container}", pod.name),
        None => pod.name,
    };
    state.shell.name(&session, id, name);
}
