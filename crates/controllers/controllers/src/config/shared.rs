//! The team's shared repo: named once it reads as a marketplace, followed after, or let go.

use groove_agent_service::shared::{self, Shared};
use groove_types::{Error, Result, SharedConfig};

use crate::{AppState, Continuation, Services, Spawner};

/// The branch followed when none is named.
const BRANCH: &str = "main";

/// The repo copied, and written to the config once its copy reads as a marketplace.
pub(super) fn join(state: &mut AppState, spawner: &dyn Spawner, (url, branch): (String, String)) {
    let url = url.trim().to_string();
    if url.is_empty() {
        return refused(state, Error::invalid("a shared repo needs its URL"));
    }
    if state.config.joining {
        return;
    }
    let branch = match branch.trim() {
        "" => BRANCH.to_string(),
        named => named.to_string(),
    };
    let same = state.config.shared().filter(|held| held.url == url);
    let enabled = same.map(|held| held.enabled.clone()).unwrap_or_default();
    let wanted = SharedConfig {
        url,
        branch,
        enabled,
    };
    state.config.joining = true;
    state.config.unshared = None;
    let data = state.env.data_dir.clone();
    let job = state.begin(format!("copying {}", wanted.url));
    spawner.spawn(Box::pin(async move {
        let copied = shared::join(&data, &wanted).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                state.config.joining = false;
                match copied {
                    Ok(copy) => joined(state, spawner, wanted, copy),
                    Err(e) => refused(state, e),
                }
            },
        ) as Continuation
    }));
}

fn joined(state: &mut AppState, spawner: &dyn Spawner, wanted: SharedConfig, copy: Shared) {
    let Some(config) = state.config.set_shared(Some(wanted)).cloned() else {
        return refused(
            state,
            Error::invalid("there is no config to change before the first run"),
        );
    };
    state.agent.shared = Some(copy);
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        return state.failed(e);
    }
    crate::agent::skills::list(state, spawner);
}

/// No shared repo any more: the config forgets it and the copy goes.
pub(super) fn leave(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(config) = state.config.set_shared(None).cloned() else {
        return;
    };
    state.agent.shared = None;
    state.config.unshared = None;
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        return state.failed(e);
    }
    let copy = shared::copy_of(&state.env.data_dir);
    if copy.exists()
        && let Err(e) = std::fs::remove_dir_all(&copy)
    {
        state.failed(Error::new(groove_types::ErrorKind::Io, e.to_string()));
    }
    crate::agent::skills::list(state, spawner);
}

/// What a follow found: the copy kept, or the last one kept and its failure said once.
pub(crate) fn followed(state: &mut AppState, read: Option<Result<Shared>>) {
    match read {
        None => state.agent.shared = None,
        Some(Ok(copy)) => {
            state.agent.shared = Some(copy);
            state.config.unshared = None;
        }
        Some(Err(e)) => {
            let again = state.config.unshared.as_deref() == Some(e.message.as_str());
            state.config.unshared = Some(e.message.clone());
            if !again {
                state.failed(e);
            }
        }
    }
}

fn refused(state: &mut AppState, e: Error) {
    state.config.unshared = Some(e.message.clone());
    state.failed(e);
}
