//! The sync running, or the last one: its progress, then each phase and wave in order, resource by resource.

use std::collections::BTreeMap;

use groove_gfx::{Edges, Rect};
use groove_types::{AppPart, Operation};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::{Panel, hairline};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Shown};

use super::super::opened::Opened;
use super::argo;
use super::grouped::{self, Columned};
use super::stages::{Stage, State, Step, stages};
use crate::ctx::Ctx;
use crate::hit::Target;

const COLUMNS: [Columned; 4] = [
    ("STATE", "✓"),
    ("KIND", "persistentvolumeclaim"),
    ("NAME", "pythie-cayzn-config-manager"),
    ("STATUS", ""),
];

enum Line<'a> {
    Heading(&'a Stage<'a>, bool),
    Step(&'a Step<'a>),
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    (tab, part): (&Opened, &AppPart),
    dragged: Option<&BTreeMap<String, f32>>,
) {
    let row = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [line, room] = column_in(body, [row, Spec::fill()]);
    let inside = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let Some(operation) = &part.operation else {
        let said = ctx.styles.small(Role::Faint);
        Label::new("no sync yet", said).draw(ctx, inside);
        return;
    };
    let mut stages = stages(part);
    for stage in &mut stages {
        grouped::order(&mut stage.steps, tab, key);
    }
    hairline(ctx, line, ctx.styles.line());
    progress(ctx, inside, (part, operation), &stages);
    let mut lines = Vec::new();
    for stage in &stages {
        let shut = tab.folded(&stage.title, stage.settled());
        lines.push(Line::Heading(stage, shut));
        if !shut {
            lines.extend(stage.steps.iter().map(Line::Step));
        }
    }
    let count = lines.len();
    let turn = groove_ui_kit::base::motion::turn(ctx.tick);
    grouped::draw(
        ctx,
        room,
        (tab, dragged),
        (&COLUMNS, count),
        |at| match &lines[at] {
            Line::Heading(stage, shut) => heading(stage, *shut, turn),
            Line::Step(one) => step(one, turn),
        },
    );
}

/// The operation's line on its colour, then a bar that fills resource by resource.
fn progress(
    ctx: &mut Ctx,
    line: Rect,
    (part, operation): (&AppPart, &Operation),
    stages: &[Stage],
) {
    if let Some(role) = argo::tint(part) {
        argo::ground(ctx, line, role);
    }
    let steps = stages.iter().flat_map(|stage| &stage.steps);
    let total = steps.clone().count().max(1);
    let done = steps.filter(|one| one.state == State::Done).count();
    let mut room = line;
    let said = format!("{done} of {total}");
    Label::new(&said, ctx.styles.small(Role::Faint)).right(ctx, &mut room, ctx.tokens.sm);
    let gap = Spec::default().width(ctx.tokens.md);
    let bar = Spec::default().width(ctx.tokens.aside_near);
    let [room, _, bar] = row_in(room, [Spec::fill(), gap, bar]);
    let thick = Spec::default().height(ctx.tokens.sm);
    let [_, track, _] = column_in(bar, [Spec::fill(), thick, Spec::fill()]);
    Panel::default().ground(ctx.styles.line()).draw(ctx, track);
    let filled = Rect {
        w: track.w * done as f32 / total as f32,
        ..track
    };
    let working = ctx.styles.color(Role::Working);
    Panel::default().ground(working).draw(ctx, filled);
    argo::operation(ctx, room, operation, ctx.now);
}

/// `▾ ⟳ wave 0 38 16 applied · 2 applying · 20 waiting`.
fn heading<'a>(stage: &'a Stage<'a>, shut: bool, turn: u8) -> Shown<'a, Target> {
    let fold = if shut { "▸" } else { "▾" };
    let (mark, role) = match () {
        _ if stage.count(State::Failed) > 0 => ("✗", Role::Bad),
        _ if stage.count(State::Applying) > 0 => ("⟳", Role::Working),
        _ if stage.count(State::Waiting) == stage.steps.len() => ("◌", Role::Ghost),
        _ if stage.count(State::Waiting) > 0 => ("⟳", Role::Working),
        _ if stage.count(State::Done) == 0 => ("○", Role::Ghost),
        _ => ("✓", Role::Ok),
    };
    let said: Vec<String> = [
        State::Failed,
        State::Done,
        State::Applying,
        State::Waiting,
        State::Skipped,
    ]
    .into_iter()
    .filter(|state| stage.count(*state) > 0)
    .map(|state| format!("{} {}", stage.count(state), word(state)))
    .collect();
    let cells = vec![
        Cell::small(fold, Role::Ghost),
        marked(mark, role, turn),
        Cell::small(stage.title.as_str(), Role::Text),
        Cell::small(stage.steps.len().to_string(), Role::Ghost),
        Cell::small(said.join(" · "), role),
    ];
    grouped::heading(&stage.title, cells)
}

fn step<'a>(one: &Step<'a>, turn: u8) -> Shown<'a, Target> {
    let (mark, role) = match one.state {
        State::Failed => ("✗", Role::Bad),
        State::Applying => ("⟳", Role::Working),
        State::Waiting => ("◌", Role::Ghost),
        State::Done => ("✓", Role::Ok),
        State::Skipped => ("○", Role::Ghost),
    };
    let said = match one.message.is_empty() {
        true => word(one.state).to_string(),
        false => format!("{} · {}", word(one.state), one.message),
    };
    let tone = if one.state == State::Failed {
        Role::Bad
    } else {
        Role::Faint
    };
    grouped::row(vec![
        marked(mark, role, turn),
        kind(one),
        Cell::small(one.name, Role::Text),
        Cell::small(said, tone),
    ])
}

/// A state's mark; the busy one turns, against the cell's start as the others stand.
fn marked(mark: &'static str, role: Role, turn: u8) -> Cell<'static> {
    match mark {
        "⟳" => Cell::small(" ", role).mark(Mark::Busy, turn, role),
        _ => Cell::small(mark, role),
    }
}

/// Its kind; a hook run inside a wave says so on a badge.
fn kind(one: &Step) -> Cell<'static> {
    let kind = one.kind.to_lowercase();
    match one.in_wave {
        true => Cell::small("Sync Hook", Role::Merged).badge(),
        false => Cell::small(kind, Role::Ghost),
    }
}

/// What a column sorts a step by; its state by how far it got.
fn key(one: &Step, column: &str) -> String {
    match column {
        "STATE" => format!("{}", one.state as u8),
        "KIND" => one.kind.to_string(),
        "STATUS" => one.message.to_string(),
        _ => one.name.to_string(),
    }
}

fn word(state: State) -> &'static str {
    match state {
        State::Failed => "failed",
        State::Applying => "applying",
        State::Waiting => "waiting",
        State::Done => "applied",
        State::Skipped => "not in this sync",
    }
}
