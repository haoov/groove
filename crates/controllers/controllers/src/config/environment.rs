//! Setup's environment check, and the `claude` sign-in it offers.

use groove_agent_service::Hooks;
use groove_config_service::Event as ConfigEvent;

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

/// One check at a time; a second ask while it runs is the same answer.
pub(super) fn check(state: &mut AppState, spawner: &dyn Spawner) {
    if state.config.checking {
        return;
    }
    state.config.checking = true;
    let home = state.env.home.clone();
    spawner.spawn(Box::pin(async move {
        let claude = groove_config_service::claude_bin(&home).await;
        let tools = groove_config_service::check(&claude).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(Event::Config(ConfigEvent::Checked(tools)), state)
        }) as Continuation
    }));
}

/// The sign-in asked last, on its own terminal; the check runs again when it exits.
pub(super) fn login(state: &mut AppState, spawner: &dyn Spawner, size: (u16, u16)) {
    if state.config.login.is_some() || state.config.signing_in {
        return;
    }
    state.config.signing_in = true;
    state.config.sign_in += 1;
    let generation = state.config.sign_in;
    let palette = groove_agent_service::palette(state.config.theme());
    let (home, hooks) = (state.env.home.clone(), hooks(spawner, generation));
    spawner.spawn(Box::pin(async move {
        let result = groove_config_service::login(&home, size, palette, hooks).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            let current = state.config.sign_in == generation;
            state.config.signing_in &= !current;
            match result {
                Ok(terminal) if current => state.config.login = Some(terminal),
                Ok(orphan) => drop(orphan.terminate()),
                Err(e) if current => state.failed(e),
                Err(_) => {}
            }
        }) as Continuation
    }));
}

pub(super) fn send(state: &AppState, bytes: &[u8]) {
    if let Some(terminal) = &state.config.login {
        let _ = terminal.write(bytes);
    }
}

pub(super) fn paste(state: &AppState, text: &str) {
    if let Some(terminal) = &state.config.login {
        let _ = terminal.paste(text);
    }
}

pub(super) fn resize(state: &AppState, (cols, rows): (u16, u16)) {
    if let Some(terminal) = &state.config.login {
        let _ = terminal.resize(cols, rows);
    }
}

pub(super) fn end(state: &mut AppState) {
    state.config.signing_in = false;
    state.config.sign_in += 1;
    if let Some(terminal) = state.config.login.take() {
        let _ = terminal.terminate();
    }
}

fn hooks(spawner: &dyn Spawner, generation: u64) -> Hooks {
    let sink = spawner.sink();
    let on_damage = Box::new(coalesced(sink.clone(), || {
        Box::new(|_: &mut AppState, _: &Services, _: &dyn Spawner| {})
    }));
    let on_exit = Box::new(move |_code| {
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if state.config.sign_in == generation {
                    state.config.login = None;
                }
                check(state, spawner);
            },
        ));
    });
    Hooks { on_damage, on_exit }
}
