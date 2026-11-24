//! A click on the rail's own parts: a session opened on its ask, a window folded, the feed narrowed.
//! A press on a session's row takes hold of it, to carry it elsewhere on the rail.

use groove_controllers::{AppState, Command, agent, session};
use groove_types::SessionId;

use crate::hit::Target;
use crate::layout::Layout;
use crate::views::rail::{landing, rows_of, slot_at, top_of};
use crate::{Held, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::CLICK_SLOP;

/// A press on a row of the rail's list holds its session; a child follows its parent instead.
pub(super) fn takes(
    ui: &mut Ui,
    app: &AppState,
    id: &SessionId,
    (x, y): (f32, f32),
    metrics: Metrics,
) {
    let tokens = metrics.tokens();
    let rows = rows_of(&tokens, Layout::of(metrics, ui).sessions);
    if !rows.contains(x, y) {
        return;
    }
    let Some(top) = top_of(&tokens, app, ui, rows, id) else {
        return;
    };
    ui.rail.drop = None;
    ui.held = Some(Held::Session {
        id: id.clone(),
        from: y,
        grip: y - top,
        at: y,
    });
}

/// The carried row follows the pointer once it has left the press, over the gap nearest it.
pub(super) fn carried(y: f32, ui: &mut Ui, app: &AppState, metrics: Metrics) {
    let Some(Held::Session { from, at, .. }) = &mut ui.held else {
        return;
    };
    *at = y;
    if ui.rail.drop.is_none() && (y - *from).abs() <= CLICK_SLOP * metrics.scale {
        return;
    }
    let tokens = metrics.tokens();
    let rows = rows_of(&tokens, Layout::of(metrics, ui).sessions);
    ui.rail.drop = slot_at(&tokens, app, ui, rows, y);
}

/// The carry let go: the session placed where it landed.
pub(super) fn dropped(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let landed = landing(app, ui);
    ui.rail.drop = None;
    let Some((session, section, index)) = landed else {
        return Vec::new();
    };
    let place = session::Command::Place {
        session,
        section,
        index,
    };
    vec![Command::Session(place)]
}

/// The rail's own targets; anything else is not its to answer.
pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    match target {
        Target::Feed => Some(folded_feed(ui)),
        Target::SessionsFold => {
            ui.rail.stowed = !ui.rail.stowed;
            Some(Vec::new())
        }
        Target::SectionFold(section) => {
            ui.rail.shut.toggle(*section);
            Some(Vec::new())
        }
        Target::FeedScope(mine) => Some(narrowed_feed(ui, *mine)),
        Target::FeedRun(key) => {
            ui.rail.open = (ui.rail.open != Some(*key)).then_some(*key);
            Some(Vec::new())
        }
        Target::RoutineRun(id) => {
            let run = agent::Command::RunRoutine { id: id.clone() };
            Some(vec![Command::Agent(run)])
        }
        Target::Session(session) => {
            let opened = super::opened_session(ui, session.clone());
            let ask = app.agent.activity(session).and_then(|one| one.asks.first());
            if let Some(ask) = ask {
                ui.examine(ask.id.clone());
            }
            Some(opened)
        }
        Target::FeedLine(session) => Some(super::opened_session(ui, session.clone())),
        Target::Approve(id) => Some(answered(ui, agent::Command::Approve { id: id.clone() })),
        Target::Refuse(id) => Some(answered(ui, agent::Command::Refuse { id: id.clone() })),
        Target::Sheet => Some(Vec::new()),
        _ => None,
    }
}

/// A write decided, and the sheet that showed it put away.
fn answered(ui: &mut Ui, answer: agent::Command) -> Vec<Command> {
    ui.close(|one| matches!(one, crate::Overlay::Examining(_)));
    vec![Command::Agent(answer)]
}

/// The feed folded to its own heading, or opened again.
fn folded_feed(ui: &mut Ui) -> Vec<Command> {
    ui.rail.folded = !ui.rail.folded;
    ui.rail.feed = 0.0;
    Vec::new()
}

/// The feed on every session, or on the one in hand.
fn narrowed_feed(ui: &mut Ui, mine: bool) -> Vec<Command> {
    ui.rail.mine = mine;
    ui.rail.feed = 0.0;
    Vec::new()
}
