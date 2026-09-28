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
    let claude = groove_agent_service::claude_bin(&state.env.home);
    spawner.spawn(Box::pin(async move {
        let tools = groove_config_service::check(&claude).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(Event::Config(ConfigEvent::Checked(tools)), state)
        }) as Continuation
    }));
}

/// The sign-in on its own terminal; the check runs again when it exits.
pub(super) fn login(state: &mut AppState, spawner: &dyn Spawner, size: (u16, u16)) {
    if state.agent.login.is_some() {
        return;
    }
    let palette = groove_agent_service::palette(state.config.theme());
    match groove_agent_service::login(&state.env.home, size, palette, hooks(spawner)) {
        Ok(terminal) => state.agent.login = Some(terminal),
        Err(e) => state.failed(e),
    }
}

pub(super) fn send(state: &AppState, bytes: &[u8]) {
    if let Some(terminal) = &state.agent.login {
        let _ = terminal.write(bytes);
    }
}

pub(super) fn paste(state: &AppState, text: &str) {
    if let Some(terminal) = &state.agent.login {
        let _ = terminal.paste(text);
    }
}

pub(super) fn resize(state: &AppState, (cols, rows): (u16, u16)) {
    if let Some(terminal) = &state.agent.login {
        let _ = terminal.resize(cols, rows);
    }
}

pub(super) fn end(state: &mut AppState) {
    if let Some(terminal) = state.agent.login.take() {
        let _ = terminal.terminate();
    }
}

fn hooks(spawner: &dyn Spawner) -> Hooks {
    let sink = spawner.sink();
    let on_damage = Box::new(coalesced(sink.clone(), || {
        Box::new(|_: &mut AppState, _: &Services, _: &dyn Spawner| {})
    }));
    let on_exit = Box::new(move |_code| {
        sink.deliver(Box::new(
            |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.agent.login = None;
                check(state, spawner);
            },
        ));
    });
    Hooks { on_damage, on_exit }
}
