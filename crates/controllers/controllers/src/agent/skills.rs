//! The skills an agent is sent: listed, written, deleted, and typed into its prompt.

use groove_agent_service::skills::{self, Dirs};
use groove_types::{SessionId, Skill};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// Where the plugins live, under the app's own directories, the shared ones as last read.
pub(crate) fn dirs(state: &AppState) -> Dirs {
    let plain = Dirs::new(&state.env.data_dir, &state.env.config_dir);
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

/// The shared copy followed and every plugin written, then every skill they offer onto the slice.
pub fn list(state: &mut AppState, spawner: &dyn Spawner) {
    let dirs = dirs(state);
    let (data, shared) = (state.env.data_dir.clone(), state.config.shared().cloned());
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
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            crate::config::shared::followed(state, followed);
            if let Err(e) = made {
                state.failed(e);
            }
            state.agent.skills = read;
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
