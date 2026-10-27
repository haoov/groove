//! A pod's containers as the list's table: a row each, unfolding into its image, probes, ports and mounts.

use groove_gfx::{Edges, Rect};
use groove_types::{Container, Pressure, State, Usage, bytes, cores, gauge, quantity};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_of, row_in};
use groove_ui_kit::shape::hung_under;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Column, Shown, Width, glass_pane};

use super::super::opened::Opened;
use super::{copy, grouped};
use crate::ctx::Ctx;
use crate::hit::Target;

/// A resource's used, request and limit, under the cell the pointer rests on.
pub(super) struct Tip {
    under: Rect,
    lines: [(&'static str, String); 3],
}

/// Folded at first unless it is not ready; init containers start folded.
pub(super) fn folded(tab: &Opened, one: &Container) -> bool {
    tab.folded(&key(one), one.init || one.ready)
}

fn key(one: &Container) -> String {
    format!("container/{}", one.name)
}

/// What an unfolded row holds, a line each; a second port or mount under an empty key.
pub(super) fn details(one: &Container) -> Vec<(&'static str, String)> {
    let mut out = vec![("image", one.image.clone())];
    for (key, probe) in [("liveness", &one.liveness), ("readiness", &one.readiness)] {
        out.extend(probe.clone().map(|probe| (key, probe)));
    }
    for (key, values) in [("ports", &one.ports), ("mounts", &one.mounts)] {
        for (at, value) in values.iter().enumerate() {
            out.push((if at == 0 { key } else { "" }, value.clone()));
        }
    }
    out
}

/// The widest usage cell, which the CPU and memory columns fit.
fn meter() -> Cell<'static> {
    Cell::body("9999Mi / 999Gi", Role::Text).bar(None, None, Role::Text)
}

/// The fold caret's column, as wide as the caret.
fn caret() -> Cell<'static> {
    Cell::fold(true, Role::Ghost)
}

fn columns<'a>((caret, meter): (&'a Cell<'a>, &'a Cell<'a>)) -> Vec<Column<'a, Target>> {
    let fit = Width::FitBody;
    vec![
        Column::plain("", Width::FitCell(caret)),
        Column::plain("NAME", fit("init · statsd-exporter")),
        Column::plain("STATE", fit("waiting · CrashLoopBackOff")),
        Column::plain("READY", Width::Fit("✓")),
        Column::plain("RESTARTS", fit("14")),
        Column::plain("LAST ENDED", fit("Error · exit 137 · 59m")),
        Column::plain("CPU", Width::FitCell(meter)),
        Column::plain("MEMORY", Width::FitCell(meter)),
        Column::plain("", Width::Fill),
    ]
}

/// The container's row, under the header when `header`; returns its usage's tip under the pointer.
pub(super) fn row(
    ctx: &mut Ctx,
    rect: Rect,
    (tab, one, header): (&Opened, &Container, bool),
    usage: Option<&Usage>,
) -> Option<Tip> {
    let used = usage.and_then(|all| all.containers.iter().find(|(name, ..)| *name == one.name));
    let (cpu, cpu_tip) = metered(&one.cpu, used.map(|(_, cpu, _)| *cpu), cores);
    let (memory, memory_tip) = metered(&one.memory, used.map(|(.., memory)| *memory), bytes);
    let (said, role) = state(one);
    let name = match one.init {
        true => format!("init · {}", one.name),
        false => one.name.clone(),
    };
    let cells = vec![
        Cell::fold(!folded(tab, one), Role::Ghost),
        Cell::body(name, if one.init { Role::Faint } else { Role::Text }),
        Cell::body(said, role),
        ready(one),
        restarts(one),
        ended(ctx, one),
        cpu,
        memory,
    ];
    let shown = Shown::row(cells).target(Target::ResourceFold(key(one), folded(tab, one)));
    let (caret, meter) = (caret(), meter());
    let columns = columns((&caret, &meter));
    let laid = grouped::single(ctx, &columns, header).one(ctx, rect, shown);
    let cell = |at: usize| laid.cell(0, at);
    let mut tip = None;
    for (at, memory, lines) in [(6, false, cpu_tip), (7, true, memory_tip)] {
        let target = Target::ContainerUsage(one.name.clone(), memory);
        if ctx.interact(cell(at), target) {
            tip = Some(Tip {
                under: cell(at),
                lines,
            });
        }
    }
    tip
}

/// One line of an unfolded row, its key under the name column; a click copies its value.
pub(super) fn detail(
    ctx: &mut Ctx,
    rect: Rect,
    (key, value): &(&'static str, String),
    last: Option<&str>,
) {
    let (caret, meter) = (caret(), meter());
    let columns = columns((&caret, &meter));
    let laid = grouped::single(ctx, &columns, false).laid(ctx, rect);
    let rect = laid.from(0, 1);
    let keyed = Spec::default().width(ctx.tokens.aside_near);
    let [k, v] = row_in(rect, [keyed, Spec::fill()]);
    Label::new(key, ctx.styles.small(Role::Ghost)).draw(ctx, k);
    copy::Copied::new(value, Role::Muted, last).draw(ctx, v);
}

/// The tip over everything drawn before it, under its cell.
pub(super) fn tip(ctx: &mut Ctx, tip: &Tip) {
    ctx.layer();
    let (sm, row) = (ctx.tokens.sm, ctx.tokens.row);
    let size = (
        ctx.tokens.aside_mid,
        row * tip.lines.len() as f32 + sm * 2.0,
    );
    let box_ = hung_under(tip.under, size);
    glass_pane(ctx, box_);
    let inside = box_.pad(Edges::all(sm)).pad(Edges::across(sm, sm));
    let lines = column_of(inside, &vec![Spec::default().height(row); tip.lines.len()]);
    for (line, (key, value)) in lines.iter().zip(&tip.lines) {
        let mut room = *line;
        Label::new(key, ctx.styles.small(Role::Ghost)).left(ctx, &mut room, sm);
        Label::new(value, ctx.styles.small(Role::Text)).right(ctx, &mut room, 0.0);
    }
}

/// The usage's bar against the limit, the request a tick; its text, and what its tip says.
fn metered(
    (request, limit): &(Option<String>, Option<String>),
    used: Option<f64>,
    write: fn(f64) -> String,
) -> (Cell<'static>, [(&'static str, String); 3]) {
    let none = || "none".to_string();
    let said = used.map(write);
    let lines = [
        ("used", said.clone().unwrap_or_else(|| "–".into())),
        ("request", request.clone().unwrap_or_else(none)),
        ("limit", limit.clone().unwrap_or_else(none)),
    ];
    let read = |one: &Option<String>| one.as_deref().and_then(quantity);
    let Some(gauge) = gauge(read(request), read(limit), used) else {
        return (Cell::small("–", Role::Ghost), lines);
    };
    let role = match gauge.pressure {
        Pressure::Under => Role::Working,
        Pressure::Over => Role::Warn,
        Pressure::Near => Role::Bad,
    };
    let text = match (said, limit) {
        (Some(used), Some(limit)) => format!("{used} / {limit}"),
        (Some(used), None) => used,
        (None, Some(limit)) => format!("– / {limit}"),
        (None, None) => "–".into(),
    };
    let part = |one: Option<f64>| one.map(|part| part as f32);
    let cell = Cell::body(text, role).bar(part(gauge.used), part(gauge.request), role);
    (cell, lines)
}

/// What the container is doing, in its colour.
fn state(one: &Container) -> (String, Role) {
    match &one.state {
        State::Running { .. } if one.ready => ("running".into(), Role::Ok),
        State::Running { .. } => ("running · not ready".into(), Role::Warn),
        State::Waiting { reason } => (format!("waiting · {reason}"), Role::Bad),
        State::Terminated { reason, exit: 0 } => (format!("terminated · {reason}"), Role::Faint),
        State::Terminated { reason, .. } => (format!("terminated · {reason}"), Role::Bad),
        State::Unknown => ("unknown".into(), Role::Faint),
    }
}

fn ready(one: &Container) -> Cell<'static> {
    match (one.init, one.ready) {
        (true, _) => Cell::small("–", Role::Ghost),
        (false, true) => Cell::small("✓", Role::Ok),
        (false, false) => Cell::small("✗", Role::Bad),
    }
}

fn restarts(one: &Container) -> Cell<'static> {
    let role = if one.restarts > 0 {
        Role::Bad
    } else {
        Role::Muted
    };
    Cell::body(one.restarts.to_string(), role)
}

/// `Error · exit 1 · 2m`.
fn ended(ctx: &Ctx, one: &Container) -> Cell<'static> {
    let Some(last) = &one.last else {
        return Cell::small("–", Role::Ghost);
    };
    let when = last
        .at
        .map(|at| {
            format!(
                " · {}",
                groove_types::human_age(at.age_at(ctx.now).as_secs())
            )
        })
        .unwrap_or_default();
    let role = if last.exit == 0 {
        Role::Muted
    } else {
        Role::Bad
    };
    Cell::body(format!("{} · exit {}{when}", last.reason, last.exit), role)
}
