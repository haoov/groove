//! The agent's model and usage in the sidebar, and the menu of the models.

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;
use groove_types::chat::{Chat, MODELS, model_name, percent, resets, tokens};
use groove_types::{SessionId, Timestamp};
use groove_ui_kit::base::style::Role;

use crate::menu::{Gauge, Of, Told};

use groove_ui_kit::base::tokens::GAUGE_HIGH as HIGH;

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

/// `opus 5.5`: the selected session's agent's model, when known.
pub(crate) fn named(app: &AppState) -> Option<String> {
    let session = app.session.selected.as_ref()?;
    let agent = app.agent.agent(session)?;
    current(app.agent.chat(session), agent).map(model_name)
}

/// `opus 5.5 · context 62%` for the selected session's agent, as far as it is known.
pub(crate) fn summary(app: &AppState) -> String {
    let session = app.session.selected.as_ref();
    let Some(agent) = session.and_then(|one| app.agent.agent(one)) else {
        return String::new();
    };
    let chat = session.and_then(|one| app.agent.chat(one));
    let model = current(chat, agent).map(model_name);
    let held = context(chat, agent).and_then(|(held, size)| Some(held as f32 / size? as f32));
    let context = held.map(|share| format!("context {}", percent(share)));
    let parts: Vec<String> = model.into_iter().chain(context).collect();
    parts.join(" · ")
}

fn gauge_role(share: f32) -> Role {
    if share >= HIGH {
        Role::Attention
    } else {
        Role::Faint
    }
}

/// The models a headless agent switches to.
pub(crate) fn models(app: &AppState, session: &SessionId) -> Option<Of> {
    let agent = app.agent.agent(session)?;
    let chat = app.agent.chat(session);
    let on = current(chat, agent);
    let switches: &[&str] = if agent.stream.is_some() { &MODELS } else { &[] };
    let rows: Vec<Told> = switches
        .iter()
        .map(|one| Told {
            label: model_name(one),
            model: Some(one.to_string()),
            current: on == Some(*one),
        })
        .collect();
    let session = session.clone();
    (!rows.is_empty()).then_some(Of::Models { session, rows })
}

/// The context, the five-hour window and the week, as far as each is known.
pub(super) fn gauges(chat: Option<&Chat>, agent: &Agent, now: Timestamp) -> Vec<Gauge> {
    let mut gauges: Vec<Gauge> = context(chat, agent)
        .into_iter()
        .filter_map(contained)
        .collect();
    let windows = [
        ("Five-hour window", agent.limits.five_hour),
        ("This week", agent.limits.week),
    ];
    let windows = windows
        .into_iter()
        .filter_map(|(label, one)| Some((label, one?)));
    gauges.extend(windows.map(|(label, one)| Gauge {
        label,
        value: percent(one.used),
        used: one.used,
        resets: one.resets.map(|at| resets(at, now)),
    }));
    gauges
}

/// `124k of 200k`; none while the window is not known.
fn contained((held, size): (u64, Option<u64>)) -> Option<Gauge> {
    let size = size?;
    Some(Gauge {
        label: "Context",
        value: format!("{} of {}", tokens(held), tokens(size)),
        used: held as f32 / size as f32,
        resets: None,
    })
}

pub(crate) use menu::draw as menu;
pub(super) use menu::gauge;

mod menu {
    //! The models' menu on glass: `MODEL` and its rows; the gauges the sidebar draws.

    use groove_gfx::{Edges, Rect};
    use groove_ui_kit::base::style::Role;
    use groove_ui_kit::shape::{band, from_top, kept_in};
    use groove_ui_kit::text::Label;
    use groove_ui_kit::widgets::{glass_chosen, glass_hoverable, glass_pane, meter};

    use crate::ctx::Ctx;
    use crate::hit::Target;
    use crate::menu::{Gauge, Menu, Told};
    use groove_ui_kit::widgets::Corner;
    use groove_ui_kit::widgets::{cap, cap_right};

    /// The menu with its corner at the menu's point, kept in the window.
    pub(crate) fn draw(ctx: &mut Ctx, open: &Menu, rows: &[Told]) {
        let row = ctx.tokens.row;
        let tall = (1 + rows.len()) as f32 * row + ctx.tokens.sm * 2.0;
        let wide = ctx.tokens.aside_far;
        let at = match open.corner {
            Corner::BottomRight => (open.at.0 - wide, open.at.1 - tall),
            Corner::BottomLeft => (open.at.0, open.at.1 - tall),
            Corner::TopRight => (open.at.0 - wide, open.at.1),
            Corner::TopLeft => open.at,
        };
        let card = kept_in(at, (wide, tall), ctx.window);
        ctx.layer();
        glass_pane(ctx, card);
        let mut room = card.pad(Edges::xy(0.0, ctx.tokens.sm));
        head(ctx, &mut room, "model");
        for (at, one) in rows.iter().enumerate() {
            let line = from_top(&mut room, row, 0.0);
            if !glass_hoverable(ctx, line, Target::MenuRow(at)) && one.current {
                glass_chosen(ctx, line);
            }
            let mut inner = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
            if one.current {
                cap_right(ctx, &mut inner, "current", Role::Accent);
            }
            Label::new(&one.label, ctx.styles.body(Role::Text)).draw(ctx, inner);
        }
    }

    fn head(ctx: &mut Ctx, room: &mut Rect, word: &str) {
        let line = from_top(room, ctx.tokens.row, 0.0);
        let mut line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        cap(ctx, &mut line, word, Role::Ghost);
    }

    /// Its label and value, the gauge under, and when it resets beside the value.
    pub(in crate::views::session::agent_pane) fn gauge(ctx: &mut Ctx, two: Rect, one: &Gauge) {
        let mut top = band(two, 0.0, ctx.tokens.row);
        let value = match &one.resets {
            Some(resets) => format!("{} · {resets}", one.value),
            None => one.value.clone(),
        };
        Label::new(&value, ctx.styles.log(Role::Muted)).right(ctx, &mut top, 0.0);
        Label::new(one.label, ctx.styles.body(Role::Text)).draw(ctx, top);
        let under = band(two, ctx.tokens.row, ctx.tokens.sm);
        meter(ctx, under, one.used, super::gauge_role(one.used));
    }
}
