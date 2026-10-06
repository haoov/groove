//! The `cluster` controller: one function per user action on the `cluster` service.

use groove_cluster_service::Event as ClusterEvent;

use crate::{AppState, Continuation, Event, Services, Spawner, apply};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `cluster.scan_contexts`: the kubeconfig files read again for the contexts they name.
    ScanContexts,
    /// `cluster.check_context`: whether one context signs in now.
    CheckContext { context: String },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::ScanContexts => "cluster.scan_contexts",
            Command::CheckContext { .. } => "cluster.check_context",
        }
    }
}

pub fn dispatch(command: Command, state: &mut AppState, _: &Services, spawner: &dyn Spawner) {
    match command {
        Command::ScanContexts => scan(state, spawner),
        Command::CheckContext { context } => check(state, spawner, context),
    }
}

/// One scan at a time; a second ask while it runs is the same answer.
fn scan(state: &mut AppState, spawner: &dyn Spawner) {
    if !state.cluster.begin_scan() {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let found = groove_cluster_service::scan(paths).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| match found {
                Ok(contexts) => {
                    apply(Event::Cluster(ClusterEvent::Found(contexts)), state);
                    check_added(state, spawner);
                }
                Err(e) => {
                    apply(Event::Cluster(ClusterEvent::Unread), state);
                    state.failed(e);
                }
            },
        ) as Continuation
    }));
}

/// Every context added to Groove, checked again.
fn check_added(state: &mut AppState, spawner: &dyn Spawner) {
    let added: Vec<String> = state
        .config
        .clusters()
        .iter()
        .map(|one| one.context.clone())
        .collect();
    for context in added {
        check(state, spawner, context);
    }
}

/// One check a context at a time.
pub(crate) fn check(state: &mut AppState, spawner: &dyn Spawner, context: String) {
    if !state.cluster.begin_check(&context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let login = groove_cluster_service::check(paths, context.clone()).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(
                Event::Cluster(ClusterEvent::Checked { context, login }),
                state,
            )
        }) as Continuation
    }));
}
