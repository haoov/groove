//! The skills an agent is sent: listed, written, deleted, and typed into its prompt.

use groove_agent_service::listing;
use groove_agent_service::skills::{self, Dirs};
use groove_types::{SessionId, Skill};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// Where the plugins live, under the app's own directories, the shared ones as last read.
pub(crate) fn dirs(state: &AppState) -> Dirs {
    let at = (state.env.data_dir.as_path(), state.env.config_dir.as_path());
    let off = state.config.skills_off().to_vec();
    let plugins = state
        .agent
        .shared
        .as_ref()
        .map(|one| one.marketplace.plugins.clone());
    listing::dirs(at, off, plugins, state.config.shared())
}

/// The shared copy followed and every plugin written, then every skill and routine onto the slice.
pub fn list(state: &mut AppState, spawner: &dyn Spawner) {
    let dirs = dirs(state);
    let at = (state.env.data_dir.clone(), state.env.config_dir.clone());
    let shared = state.config.shared().cloned();
    let held = state.agent.shared.as_ref().map(|one| one.path.clone());
    spawner.spawn(Box::pin(async move {
        let found = listing::listing(dirs, at, shared, held).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            crate::config::shared::followed(state, found.followed);
            if let Err(e) = found.made {
                state.failed(e);
            }
            state.agent.skills = found.skills;
            state.agent.routines = found.routines;
            state.agent.listed = true;
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
    entered(spawner, session);
}

/// Words pasted into the agent's prompt, then the return that submits them.
pub(crate) fn typed(state: &mut AppState, spawner: &dyn Spawner, session: &SessionId, words: &str) {
    if let Some(terminal) = state.agent.terminal(session) {
        let _ = terminal.paste(words);
    }
    entered(spawner, session);
}

fn entered(spawner: &dyn Spawner, session: &SessionId) {
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
pub fn delete(state: &mut AppState, spawner: &dyn Spawner, id: &str) {
    let own = state
        .agent
        .skills
        .iter()
        .find(|one| one.id == id && one.editable);
    let Some(name) = own.map(|one| one.name.clone()) else {
        let why = format!("`{id}` is no skill of your own to delete");
        return state.failed(groove_types::Error::invalid(why));
    };
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
    if !crate::config::written(state, Some(config)) {
        return;
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
