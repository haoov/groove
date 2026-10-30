//! The skills an agent is sent: listed, written, deleted, and typed into its prompt.

use groove_skill_service::listing;
use groove_skill_service::skills::{self, Dirs};
use groove_types::{SessionId, Skill};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// Where the plugins live, under the app's own directories, the shared ones as last read.
pub(crate) fn dirs(state: &AppState) -> Dirs {
    let at = (state.env.data_dir.as_path(), state.env.config_dir.as_path());
    let off = state.config.skills_off().to_vec();
    let plugins = state
        .skill
        .shared
        .as_ref()
        .map(|one| one.marketplace.plugins.clone());
    listing::dirs(at, off, plugins, state.config.shared())
}

/// The shared copy followed and every plugin written, then every skill and routine onto the slice.
pub fn list(state: &mut AppState, spawner: &dyn Spawner) {
    if state.skill.listing {
        state.skill.again = true;
        return;
    }
    if let Some(wanted) = state.skill.queued.take() {
        state.skill.again = true;
        return crate::config::shared::copy(state, spawner, wanted);
    }
    if state.skill.joining {
        state.skill.again = true;
        return;
    }
    state.skill.listing = true;
    let generation = state.skill.generation;
    let dirs = dirs(state);
    let at = (state.env.data_dir.clone(), state.env.config_dir.clone());
    let shared = state.config.shared().cloned();
    let held = state.skill.shared.as_ref().map(|one| one.path.clone());
    spawner.spawn(Box::pin(async move {
        let found = listing::listing(dirs, at, shared, held).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                listed(state, (services, spawner), generation, found)
            },
        ) as Continuation
    }));
}

/// A listing landed: taken unless the shared repo changed meanwhile; then what waited on it.
fn listed(
    state: &mut AppState,
    (services, spawner): (&Services, &dyn Spawner),
    generation: u64,
    found: listing::Listing,
) {
    state.skill.listing = false;
    match state.skill.generation == generation {
        true => {
            crate::config::shared::followed(state, found.followed);
            if let Err(e) = found.made {
                state.failed(e);
            }
            relisted(state, found.skills);
            state.routine.routines = found.routines;
            state.skill.listed = true;
        }
        false => state.skill.again = true,
    }
    if std::mem::take(&mut state.skill.unsharing) {
        return crate::config::shared::forget(state, services, spawner);
    }
    if std::mem::take(&mut state.skill.again) || state.skill.queued.is_some() {
        list(state, spawner);
    }
}

/// The skills read again, or the last list kept and why said.
pub(crate) fn relisted(state: &mut AppState, read: groove_types::Result<Vec<Skill>>) {
    match read {
        Ok(skills) => state.skill.skills = skills,
        Err(e) => state.failed(e),
    }
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
    if let Some(stream) = state.agent.stream(session) {
        let _ = stream.say(said.trim_end());
        return;
    }
    super::send(state, session, said.as_bytes());
    entered(spawner, session);
}

/// Words said to a headless agent, or pasted into its terminal's prompt and submitted.
pub(crate) fn typed(state: &mut AppState, spawner: &dyn Spawner, session: &SessionId, words: &str) {
    if let Some(stream) = state.agent.stream(session) {
        let _ = stream.say(words);
        return;
    }
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
    state.skill.generation += 1;
    let dirs = dirs(state);
    let job = state.begin(format!("writing the skill {name}"));
    spawner.spawn(Box::pin(async move {
        let made = skills::save(&dirs, &name, &body, instead_of.as_deref()).await;
        let read = skills::list(&dirs).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            relisted(state, read);
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
        .skill
        .skills
        .iter()
        .find(|one| one.id == id && one.editable);
    let Some(name) = own.map(|one| one.name.clone()) else {
        let why = format!("`{id}` is no skill of your own to delete");
        return state.failed(groove_types::Error::invalid(why));
    };
    state.skill.generation += 1;
    let dirs = dirs(state);
    let job = state.begin(format!("deleting the skill {name}"));
    spawner.spawn(Box::pin(async move {
        let done = skills::delete(&dirs, &name).await;
        let read = skills::list(&dirs).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            relisted(state, read);
            if let Err(e) = done {
                state.failed(e);
            }
        }) as Continuation
    }));
}

/// One skill switched on or off, the plugins built again, and running agents marked stale.
pub fn switch(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, on): (String, bool),
) {
    let config = match state.config.switch_skill(&id, on) {
        Ok(config) => config.clone(),
        Err(e) => return state.failed(e),
    };
    crate::config::write(state, services, spawner, config, None);
    state.skill.switched = groove_types::Timestamp::now();
    state.skill.generation += 1;
    let dirs = dirs(state);
    spawner.spawn(Box::pin(async move {
        let made = skills::sync(&dirs).await;
        let read = skills::list(&dirs).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = made {
                state.failed(e);
            }
            relisted(state, read);
        }) as Continuation
    }));
}
