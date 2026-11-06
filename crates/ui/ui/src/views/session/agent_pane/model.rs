//! Two round glass buttons at the prompt's right: the agent's model, whose menu switches a headless
//! agent to another, and its context's ring, whose menu says what the agent uses.

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;
use groove_gfx::Rect;
use groove_types::chat::{Chat, MODELS, Window, model_name, percent, resets, tokens};
use groove_types::{SessionId, Timestamp};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, strip};
use groove_ui_kit::widgets::{glass_button, glass_gauge};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::menu::{Of, Told};

/// The model the agent answers with: the one switched to, else the transcript's last.
fn current<'a>(chat: Option<&'a Chat>, agent: &'a Agent) -> Option<&'a str> {
    agent.chosen.as_deref().or(chat.and_then(Chat::model))
}

/// The tokens the context holds, and how many it can hold when the stream said.
fn context(chat: Option<&Chat>, agent: &Agent) -> Option<(u64, Option<u64>)> {
    let model = current(chat, agent);
    let window = |model: &str| agent.windows.iter().find(|(one, _)| one == model);
    let size = model.and_then(window).map(|(_, size)| *size);
    chat.and_then(Chat::context).map(|held| (held, size))
}

/// At the prompt's right, on its last row: the model's button, then the context's with its ring.
pub(super) fn draw(ctx: &mut Ctx, chat: Option<&Chat>, agent: &Agent, prompt: Rect) {
    let (pane, sm, size) = (ctx.app.layout.agent, ctx.tokens.sm, ctx.tokens.bar);
    let middle = prompt.bottom() - ctx.tokens.md - ctx.tokens.code_line / 2.0;
    let line = band(pane, middle - size / 2.0 - pane.y, size);
    let model = strip(line, prompt.right() + sm - pane.x, size);
    let usage = strip(line, model.right() + sm - pane.x, size);
    if agent.stream.is_some() || current(chat, agent).is_some() {
        glass_button(ctx, model, (Mark::Agent, Role::Text), Target::ModelButton);
    }
    let share = context(chat, agent).and_then(|(held, size)| Some(held as f32 / size? as f32));
    glass_gauge(ctx, usage, share.unwrap_or_default(), Target::UsageButton);
}

/// The model's menu: the models a headless agent switches to, the current one marked; else the
/// one it answers with.
pub(crate) fn models(app: &AppState, session: &SessionId) -> Option<Of> {
    let agent = app.agent.agent(session)?;
    let on = current(app.agent.chat(session), agent);
    let switches: &[&str] = if agent.stream.is_some() { &MODELS } else { &[] };
    let mut rows: Vec<Told> = switches
        .iter()
        .map(|one| Told {
            label: model_name(one),
            model: Some(one.to_string()),
            current: on == Some(*one),
        })
        .collect();
    if switches.is_empty() {
        rows.extend(on.map(|one| said(model_name(one))));
    }
    let session = session.clone();
    (!rows.is_empty()).then_some(Of::Models { session, rows })
}

/// The usage's menu: the context, then the plan's five-hour and weekly windows.
pub(crate) fn usage(app: &AppState, session: &SessionId, now: Timestamp) -> Option<Of> {
    let agent = app.agent.agent(session)?;
    let chat = app.agent.chat(session);
    let mut rows: Vec<Told> = Vec::new();
    rows.extend(context(chat, agent).map(|(held, size)| said(contained(held, size))));
    let windows = [
        ("5 hours", agent.limits.five_hour),
        ("week", agent.limits.week),
    ];
    let windows = windows
        .into_iter()
        .filter_map(|(word, one)| one.map(|one| (word, one)));
    rows.extend(windows.map(|(word, one)| said(windowed(word, one, now))));
    let session = session.clone();
    (!rows.is_empty()).then_some(Of::Models { session, rows })
}

fn said(label: String) -> Told {
    Told {
        label,
        model: None,
        current: false,
    }
}

/// `context  82k of 1M · 8%`, or the tokens alone while the window is not known.
fn contained(held: u64, size: Option<u64>) -> String {
    match size {
        Some(size) => format!(
            "context  {} of {} · {}",
            tokens(held),
            tokens(size),
            percent(held as f32 / size as f32)
        ),
        None => format!("context  {}", tokens(held)),
    }
}

/// `5 hours  48% · resets in 1h 42m`.
fn windowed(word: &str, window: Window, now: Timestamp) -> String {
    let used = percent(window.used);
    match window.resets {
        Some(at) => format!("{word}  {used} · {}", resets(at, now)),
        None => format!("{word}  {used}"),
    }
}
