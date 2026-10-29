//! What a press on the agent's pane does: its screen, its face, its conversation and its prompt.

use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, Edit, Selection};

use crate::hit::{Hits, Target, read};
use crate::input::selection::{agent_cell, pressed as pressed_on};
use crate::{Grid, Held, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// The agent's own targets; anything else is not its to answer.
pub(super) fn acted(
    target: &Target,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    match target {
        Target::Agent => Some(pressed(ui, app, point, metrics)),
        Target::AgentView => {
            ui.agent.terminal = !ui.agent.terminal;
            Some(Vec::new())
        }
        Target::Chat => Some(reading(ui, hits, point)),
        Target::Prompt => Some(typing(ui, hits, point, metrics)),
        Target::Commands(id) => {
            let opened = &mut ui.agent.opened;
            match opened.iter().position(|one| one == id) {
                Some(at) => _ = opened.remove(at),
                None => opened.push(id.clone()),
            }
            Some(Vec::new())
        }
        Target::Edited { path, line } => Some(crate::input::reveal::opened(ui, app, path, *line)),
        _ => None,
    }
}

/// A press on the conversation: a selection begins, or takes the word or run under a 2nd or 3rd click.
fn reading(ui: &mut Ui, hits: &Hits, point: (f32, f32)) -> Vec<Command> {
    let count = ui.clicked.map_or(1, |click| click.count);
    let (from, to) = match count {
        1 => (point, point),
        count => read::around(hits.read(), point, count).unwrap_or((point, point)),
    };
    ui.agent.reading = Some((ui.agent.unscrolled(from), ui.agent.unscrolled(to)));
    ui.held = Some(Held::Reading);
    Vec::new()
}

/// A press on the prompt: the caret under it, then the word or the line under a second or third click.
fn typing(ui: &mut Ui, hits: &Hits, point: (f32, f32), metrics: Metrics) -> Vec<Command> {
    let Some(caret) = caret_at(hits, point, metrics) else {
        return Vec::new();
    };
    let prompt = &mut ui.agent.prompt;
    prompt.buffer.follow(caret);
    prompt.scroll = hits.typed().scroll;
    match ui.clicked.map_or(1, |click| click.count) {
        1 => ui.held = Some(Held::Prompt),
        2 => _ = prompt.buffer.edit(&Edit::SelectWord),
        _ => _ = prompt.buffer.edit(&Edit::SelectLine),
    }
    Vec::new()
}

/// The pointer moved with the conversation or the prompt held: the selection follows it.
pub(super) fn dragged(ui: &mut Ui, hits: &Hits, point: (f32, f32), metrics: Metrics) {
    match ui.held {
        Some(Held::Reading) => {
            let to = ui.agent.unscrolled(point);
            if let Some((_, head)) = ui.agent.reading.as_mut() {
                *head = to;
            }
        }
        Some(Held::Prompt) => {
            let Some(head) = caret_at(hits, point, metrics) else {
                return;
            };
            let buffer = &mut ui.agent.prompt.buffer;
            let anchor = buffer.selections().first().map_or(head, |one| one.anchor);
            buffer.holding(Selection { anchor, head });
        }
        _ => {}
    }
}

/// The conversation's text the drag held, to the clipboard.
pub(in crate::input) fn read_out(ui: &Ui, hits: &Hits) -> Vec<Command> {
    let Some(points) = ui.agent.held() else {
        return Vec::new();
    };
    let lines = hits.read();
    let text = read::text(lines, &read::held(lines, points));
    if text.is_empty() {
        return Vec::new();
    }
    vec![Command::Workspace(workspace::Command::CopyText { text })]
}

/// The place in the prompt's buffer under a point: on the row there, else the nearest row.
fn caret_at(hits: &Hits, (x, y): (f32, f32), metrics: Metrics) -> Option<Caret> {
    let rows = &hits.typed().rows;
    let below = rows.iter().position(|(rect, _)| y < rect.bottom());
    let (rect, (line, start, end)) = rows[below.unwrap_or(rows.len().checked_sub(1)?)];
    let across = ((x - rect.x) / metrics.advance).round().max(0.0) as usize;
    Some(Caret::new(line, (start + across).min(end)))
}

/// A press on the screen, through the grid's own rule.
fn pressed(ui: &mut Ui, app: &AppState, point: (f32, f32), metrics: Metrics) -> Vec<Command> {
    let cell = agent_cell(point, ui, metrics);
    pressed_on(ui, app, Grid::Agent, cell)
}
