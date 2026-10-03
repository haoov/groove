//! A task source turned on once it answers, or turned off; the tasks read again after.

use std::future::Future;

use groove_config_service::Source;
use groove_types::{Error, ProviderId, Result, Secret};

use crate::{AppState, Continuation, Services, Spawner};

/// Notion on, once the token reads the database; your tasks only, so the user is needed.
pub(super) fn notion(
    state: &mut AppState,
    spawner: &dyn Spawner,
    (token, database, user): (Secret, String, String),
) {
    let asked = [token.expose(), &database, &user];
    if asked.iter().any(|one| one.trim().is_empty()) {
        let e = Error::invalid("a Notion token, a database id and your user id are all needed");
        return refused(state, e);
    }
    connect(state, spawner, ProviderId::Notion, async move {
        let config = groove_task_service::connect_notion(token.expose(), &database, &user).await?;
        Ok(Source::Notion(Some(config)))
    });
}

/// GitHub on, once the host answers the token `gh` holds for it.
pub(super) fn github(state: &mut AppState, spawner: &dyn Spawner, host: String) {
    if host.trim().is_empty() {
        return refused(state, Error::invalid("a GitHub host is needed"));
    }
    connect(state, spawner, ProviderId::Github, async move {
        let config = groove_task_service::connect_github(&host).await?;
        Ok(Source::Github(Some(config)))
    });
}

pub(super) fn off(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: ProviderId,
) {
    let source = match id {
        ProviderId::Notion => Source::Notion(None),
        ProviderId::Github => Source::Github(None),
    };
    set(state, services, spawner, source);
}

/// One connect at a time; what it reached is held and written, what it could not is said.
fn connect(
    state: &mut AppState,
    spawner: &dyn Spawner,
    id: ProviderId,
    reach: impl Future<Output = Result<Source>> + Send + 'static,
) {
    if state.config.connecting.is_some() {
        return;
    }
    state.config.connecting = Some(id);
    state.config.refused = None;
    spawner.spawn(Box::pin(async move {
        let reached = reach.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.config.connecting = None;
                match reached {
                    Ok(source) => set(state, services, spawner, source),
                    Err(e) => refused(state, e),
                }
            },
        ) as Continuation
    }));
}

/// The block replaced and written, then every source's tasks read again.
fn set(state: &mut AppState, services: &Services, spawner: &dyn Spawner, source: Source) {
    let config = match state.config.set_source(source) {
        Ok(config) => config.clone(),
        Err(e) => return refused(state, e),
    };
    state.config.refused = None;
    if !super::written(state, Some(config)) {
        return;
    }
    crate::task::load(state, services, spawner);
}

fn refused(state: &mut AppState, e: Error) {
    state.config.refused = Some(e.message.clone());
    state.failed(e);
}
