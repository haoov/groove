//! The team's shared repo: named once it reads as a marketplace, followed after, or let go.

use groove_skill_service::shared::{self, Shared};
use groove_types::{Error, Result, SharedConfig};

use crate::{AppState, Continuation, Services, Spawner};

/// The repo copied, and written to the config once its copy reads as a marketplace.
pub(super) fn join(state: &mut AppState, spawner: &dyn Spawner, (url, branch): (String, String)) {
    let wanted = match state.config.wanted_shared(&url, &branch) {
        Ok(wanted) => wanted,
        Err(e) => return refused(state, e),
    };
    if state.skill.joining {
        return;
    }
    state.skill.joining = true;
    state.skill.unshared = None;
    match state.skill.listing {
        true => state.skill.queued = Some(wanted),
        false => copy(state, spawner, wanted),
    }
}

/// The repo copied in a job, alone on the shared copy.
pub(crate) fn copy(state: &mut AppState, spawner: &dyn Spawner, wanted: SharedConfig) {
    let data = state.env.data_dir.clone();
    let job = state.begin(format!("copying {}", wanted.url));
    spawner.spawn(Box::pin(async move {
        let copied = shared::join(&data, &wanted).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                state.skill.joining = false;
                match copied {
                    Ok(copy) => joined(state, services, spawner, (wanted, copy)),
                    Err(e) => refused_listing(state, spawner, e),
                }
                if std::mem::take(&mut state.skill.unsharing) {
                    forget(state, services, spawner);
                }
            },
        ) as Continuation
    }));
}

fn joined(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (wanted, copy): (SharedConfig, Shared),
) {
    let Some(config) = state.config.set_shared(Some(wanted)).cloned() else {
        let first = Error::invalid("there is no config to change before the first run");
        return refused_listing(state, spawner, first);
    };
    state.skill.again = false;
    state.skill.reshared(Some(copy));
    super::write(state, services, spawner, config, None);
    crate::agent::skills::list(state, spawner);
}

/// No shared repo any more: the config forgets it, and the copy goes once that is on disk.
pub(super) fn leave(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let Some(config) = state.config.set_shared(None).cloned() else {
        return;
    };
    state.skill.reshared(None);
    state.skill.unshared = None;
    super::write(state, services, spawner, config, Some(Box::new(forget)));
}

/// The copy taken away once no listing or copy is out and no repo was joined again.
pub(crate) fn forget(state: &mut AppState, _: &Services, spawner: &dyn Spawner) {
    if state.config.shared().is_some() {
        return;
    }
    if state.skill.listing || state.skill.joining {
        state.skill.unsharing = true;
        return;
    }
    let data = state.env.data_dir.clone();
    spawner.spawn(Box::pin(async move {
        let gone = shared::forget(&data).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if let Err(e) = gone {
                    state.failed(e);
                }
                crate::agent::skills::list(state, spawner);
            },
        ) as Continuation
    }));
}

/// What a follow found: the copy kept, or the last one kept and its failure said once.
pub(crate) fn followed(state: &mut AppState, read: Option<Result<Shared>>) {
    if let Some(e) = state.skill.followed(read) {
        state.failed(e);
    }
}

/// A join refused, then the listing asked for while it was out.
fn refused_listing(state: &mut AppState, spawner: &dyn Spawner, e: Error) {
    refused(state, e);
    if std::mem::take(&mut state.skill.again) {
        crate::agent::skills::list(state, spawner);
    }
}

fn refused(state: &mut AppState, e: Error) {
    state.skill.unshared = Some(e.message.clone());
    state.failed(e);
}
