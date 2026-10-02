//! The skills an agent is sent: listed, written, deleted, and typed into its prompt.

use groove_agent_service::routines;
use groove_agent_service::skills::{self, Dirs};
use groove_types::{SessionId, Skill};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// Where the plugins live, under the app's own directories, the shared ones as last read.
pub(crate) fn dirs(state: &AppState) -> Dirs {
    let off = state.config.skills_off().to_vec();
    let plain = Dirs::new(&state.env.data_dir, &state.env.config_dir).switched(off);
    let plugins = state
        .agent
        .shared
        .as_ref()
        .map(|one| one.marketplace.plugins.clone());
    sharing(plain, plugins, state.config.shared())
}

fn sharing(
    dirs: Dirs,
    plugins: Option<Vec<skills::Plugin>>,
    shared: Option<&groove_types::SharedConfig>,
) -> Dirs {
    let enabled = shared.map(|one| one.enabled.clone()).unwrap_or_default();
    dirs.sharing(plugins.unwrap_or_default(), enabled)
}

/// The shared copy followed and every plugin written, then every skill and routine onto the slice.
pub fn list(state: &mut AppState, spawner: &dyn Spawner) {
    let dirs = dirs(state);
    let (data, shared) = (state.env.data_dir.clone(), state.config.shared().cloned());
    let config = state.env.config_dir.clone();
    let held = state.agent.shared.as_ref().map(|one| one.path.clone());
    spawner.spawn(Box::pin(async move {
        let followed = match &shared {
            Some(shared) => Some(groove_agent_service::shared::follow(&data, shared).await),
            None => None,
        };
        let dirs = match &followed {
            Some(Ok(copy)) => sharing(
                dirs,
                Some(copy.marketplace.plugins.clone()),
                shared.as_ref(),
            ),
            _ => dirs,
        };
        let made = skills::sync(&dirs);
        let read = skills::list(&dirs);
        let copy = match (&followed, &held) {
            (Some(Ok(one)), _) => Some(one.path.as_path()),
            (Some(Err(_)), held) => held.as_deref(),
            (None, _) => None,
        };
        let routines = routines::list(&routines::Dirs::new(&config, copy));
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            crate::config::shared::followed(state, followed);
            if let Err(e) = made {
                state.failed(e);
            }
            state.agent.skills = read;
            state.agent.routines = routines;
        }) as Continuation
    }));
}

/// One skill typed into the agent's prompt, then the return a beat later, once its menu settles.
pub fn send(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: &SessionId,
    id: &str,
    args: Option<&str>,
) {
    let said = format!("/{id} {}", args.unwrap_or_default());
    super::send(state, session, said.as_bytes());
    let session = session.clone();
    spawner.spawn(Box::pin(async move {
        tokio::time::sleep(SETTLES).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            super::send(state, &session, b"\r");
        }) as Continuation
    }));
}

/// How long the agent's own menu takes to settle before the return lands.
const SETTLES: std::time::Duration = std::time::Duration::from_millis(120);

/// One skill of the user's own written, and the list read again.
pub fn save(
    state: &mut AppState,
    spawner: &dyn Spawner,
    name: String,
    body: String,
    instead_of: Option<String>,
    asker: Asker,
) {
    let dirs = dirs(state);
    let job = state.begin(format!("writing the skill {name}"));
    spawner.spawn(Box::pin(async move {
        let made = skills::save(&dirs, &name, &body, instead_of.as_deref());
        let read = skills::list(&dirs);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.agent.skills = read;
            let said = made.as_ref().ok().map(said);
            asker.answer(state, made.map(drop), || said.unwrap_or_default());
        }) as Continuation
    }));
}

fn said(one: &Skill) -> String {
    format!("wrote the skill {}", one.id)
}

/// One skill of the user's own deleted, and the list read again.
pub fn delete(state: &mut AppState, spawner: &dyn Spawner, name: String) {
    let dirs = dirs(state);
    let job = state.begin(format!("deleting the skill {name}"));
    spawner.spawn(Box::pin(async move {
        let done = skills::delete(&dirs, &name);
        let read = skills::list(&dirs);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.agent.skills = read;
            if let Err(e) = done {
                state.failed(e);
            }
        }) as Continuation
    }));
}

/// One skill switched on or off, the plugins built again, and running agents marked stale.
pub fn switch(state: &mut AppState, spawner: &dyn Spawner, id: String, on: bool) {
    if id.starts_with("groove:") {
        return state.failed(groove_types::Error::invalid("a core skill is always on"));
    }
    let Some(config) = state.config.switch_skill(&id, on).cloned() else {
        let why = format!("`{id}` is no skill Groove can switch: no shared repo holds it");
        return state.failed(groove_types::Error::invalid(why));
    };
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        return state.failed(e);
    }
    state.agent.switched = groove_types::Timestamp::now();
    let dirs = dirs(state);
    spawner.spawn(Box::pin(async move {
        let made = skills::sync(&dirs);
        let read = skills::list(&dirs);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = made {
                state.failed(e);
            }
            state.agent.skills = read;
        }) as Continuation
    }));
}
