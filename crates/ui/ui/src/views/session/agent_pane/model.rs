//! The line under the prompt, and the menu it opens: the models, then the usage gauges.

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;
use groove_gfx::Rect;
use groove_types::chat::{Chat, MODELS, Window, model_name, percent, resets, tokens};
use groove_types::{SessionId, Timestamp};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::from_left;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{glass_hoverable, meter};

use super::parts::{cap, cap_right};
use crate::ctx::Ctx;
use crate::hit::Target;
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

/// `CONTEXT 62% ▬▬──   5H WINDOW 34% · resets in 2h 14m`, the model at the right.
pub(super) fn line(ctx: &mut Ctx, chat: Option<&Chat>, agent: &Agent, rect: Rect) {
    glass_hoverable(ctx, rect, Target::ModelButton);
    let mut room = rect;
    if let Some(model) = current(chat, agent) {
        cap_right(ctx, &mut room, &model_name(model), Role::Faint);
    }
    let share = context(chat, agent).and_then(|(held, size)| Some(held as f32 / size? as f32));
    if let Some(share) = share {
        cap(ctx, &mut room, "context", Role::Ghost);
        let said = percent(share);
        Label::new(&said, ctx.styles.log(Role::Faint)).left(ctx, &mut room, ctx.tokens.xs);
        let gauge = from_left(&mut room, ctx.tokens.row * 2.0, ctx.tokens.lg);
        meter(ctx, gauge, share, gauge_role(share));
    }
    if let Some(five) = agent.limits.five_hour {
        cap(ctx, &mut room, "5h window", Role::Ghost);
        let said = windowed(five, ctx.now);
        Label::new(&said, ctx.styles.log(Role::Faint)).draw(ctx, room);
    }
}

fn gauge_role(share: f32) -> Role {
    if share >= HIGH { Role::Attention } else { Role::Faint }
}

/// The models a headless agent switches to, then the context and the plan's windows.
pub(crate) fn models(app: &AppState, session: &SessionId, now: Timestamp) -> Option<Of> {
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
    let mut gauges: Vec<Gauge> = context(chat, agent).into_iter().filter_map(contained).collect();
    let windows = [("Five-hour window", agent.limits.five_hour), ("This week", agent.limits.week)];
    let windows = windows.into_iter().filter_map(|(label, one)| Some((label, one?)));
    gauges.extend(windows.map(|(label, one)| Gauge {
        label,
        value: percent(one.used),
        used: one.used,
        resets: one.resets.map(|at| resets(at, now)),
    }));
    let session = session.clone();
    (!rows.is_empty() || !gauges.is_empty()).then_some(Of::Models { session, rows, gauges })
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

/// `34% · resets in 1h 42m`.
fn windowed(window: Window, now: Timestamp) -> String {
    let used = percent(window.used);
    match window.resets {
        Some(at) => format!("{used} · {}", resets(at, now)),
        None => used,
    }
}

pub(crate) use menu::draw as menu;

mod menu {
    //! The models' menu on glass: `MODEL` and its rows, then `USAGE` and its gauges.

    use groove_gfx::Edges;
    use groove_ui_kit::base::style::Role;
    use groove_ui_kit::shape::{band, from_top, kept_in};
    use groove_ui_kit::text::Label;
    use groove_ui_kit::widgets::{glass_chosen, glass_hoverable, glass_pane, meter};

    use super::super::parts::{cap, cap_right};
    use crate::ctx::Ctx;
    use crate::hit::Target;
    use crate::menu::{Gauge, Menu, Told};
    use groove_ui_kit::widgets::Corner;

    /// The menu with its corner at the menu's point, kept in the window.
    pub(crate) fn draw(ctx: &mut Ctx, open: &Menu, (rows, gauges): (&[Told], &[Gauge])) {
        let row = ctx.tokens.row;
        let heads = usize::from(!rows.is_empty()) + usize::from(!gauges.is_empty());
        let tall = (heads + rows.len() + gauges.len() * 2) as f32 * row + ctx.tokens.sm * 2.0;
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
        if !rows.is_empty() {
            head(ctx, &mut room, "model");
        }
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
        if !gauges.is_empty() {
            head(ctx, &mut room, "usage");
        }
        for one in gauges {
            let two = from_top(&mut room, row * 2.0, 0.0);
            gauge(ctx, two.pad(Edges::across(ctx.tokens.md, ctx.tokens.md)), one);
        }
    }

    fn head(ctx: &mut Ctx, room: &mut groove_gfx::Rect, word: &str) {
        let line = from_top(room, ctx.tokens.row, 0.0);
        let mut line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        cap(ctx, &mut line, word, Role::Ghost);
    }

    /// Its label and value, the gauge under, and when it resets beside the value.
    fn gauge(ctx: &mut Ctx, two: groove_gfx::Rect, one: &Gauge) {
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
