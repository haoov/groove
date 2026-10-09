//! The pod's container card: a selector over its containers, then the one picked and its gauges.

use groove_gfx::{Align, Edges, Rect};
use groove_types::{Container, PodPart, Pressure, State, Usage, bytes, cores, gauge, quantity};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, column_of, row_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{Badge, Tab, Text};

use super::super::opened::Opened;
use super::copy;
use crate::ctx::Ctx;
use crate::hit::Target;

/// How many rows a gauge stands on.
const GAUGE_ROWS: usize = 7;

/// The container the card shows: the one picked, else the first that is not an init one.
fn picked<'a>(pod: &'a PodPart, tab: &Opened) -> Option<&'a Container> {
    let named = tab.container.as_ref();
    let chosen = named.and_then(|name| pod.containers.iter().find(|one| &one.name == name));
    chosen
        .or_else(|| pod.containers.iter().find(|one| !one.init))
        .or(pod.containers.first())
}

pub(super) fn height(ctx: &Ctx, pod: &PodPart, tab: &Opened) -> f32 {
    let lines = picked(pod, tab).map_or(0, |one| {
        let (state, wiring) = facts(one, ctx);
        state.len().max(wiring.len())
    });
    let rows = 2 + lines.max(GAUGE_ROWS);
    ctx.tokens.row * rows as f32 + ctx.tokens.md * 2.0
}

pub(super) fn draw(
    ctx: &mut Ctx,
    rect: Rect,
    (tab, pod): (&Opened, &PodPart),
    usage: Option<&Usage>,
) {
    let panel = Panel::default()
        .ground(ctx.styles.band())
        .border(ctx.styles.line());
    panel.radius(ctx.tokens.corner).draw(ctx, rect);
    let inner = rect.pad(Edges::all(ctx.tokens.md));
    let row = Spec::default().height(ctx.tokens.row);
    let [strip, head, body] = column_in(inner, [row, row, Spec::fill()]);
    let Some(shown) = picked(pod, tab) else {
        return;
    };
    selector(ctx, strip, pod, shown);
    let last = tab.copied.as_deref();
    headed(ctx, head, shown, last);
    let gauge_wide = Spec::default().width(ctx.tokens.aside_far);
    let [left, right, gauges] = row_in(body, [Spec::fill(), Spec::fill(), gauge_wide]);
    let (state, wiring) = facts(shown, ctx);
    facts_column(ctx, left, &state, (false, last));
    facts_column(ctx, right, &wiring, (true, last));
    groove_ui_kit::shape::side_rule(ctx, gauges, gauges.x, ctx.styles.line());
    let gauges = gauges.pad(Edges::across(ctx.tokens.md, 0.0));
    let used = usage.and_then(|one| one.containers.iter().find(|(name, ..)| *name == shown.name));
    let fixed = Spec::default().height(ctx.tokens.row * GAUGE_ROWS as f32);
    let [gauges, _] = column_in(gauges, [fixed, Spec::fill()]);
    let [cpu, memory] = row_in(gauges, [Spec::fill(), Spec::fill()]);
    meter(
        ctx,
        cpu,
        ("cpu", &shown.cpu),
        used.map(|(_, cpu, _)| *cpu),
        cores,
    );
    meter(
        ctx,
        memory,
        ("memory", &shown.memory),
        used.map(|(_, _, memory)| *memory),
        bytes,
    );
}

/// The container's state, then its image.
fn headed(ctx: &mut Ctx, head: Rect, shown: &Container, last: Option<&str>) {
    let (said, role) = state(shown);
    let badge = Badge::new(&said, role).at(ctx, head, head.x);
    let image = Rect {
        x: badge.right() + ctx.tokens.md,
        w: head.right() - badge.right() - ctx.tokens.md,
        ..head
    };
    copy::Copied::new(&shown.image, Role::Muted, last).draw(ctx, image);
}

/// A tab for each container, the init ones named so.
fn selector(ctx: &mut Ctx, strip: Rect, pod: &PodPart, shown: &Container) {
    let mut room = strip;
    for one in &pod.containers {
        let label = match one.init {
            true => format!("init · {}", one.name),
            false => one.name.clone(),
        };
        let target = Target::ResourceContainer(one.name.clone());
        Tab::new(&label, target, one.name == shown.name)
            .text(Text::Small)
            .tight()
            .left(ctx, &mut room, 0.0);
    }
}

/// The facts a line each; where `copies`, a click copies each value.
fn facts_column(ctx: &mut Ctx, left: Rect, facts: &[Fact], (copies, last): (bool, Option<&str>)) {
    let row = Spec::default().height(ctx.tokens.row);
    let rows = column_of(left, &vec![row; facts.len()]);
    for (line, (key, value, role)) in rows.iter().zip(facts) {
        let [k, v] = row_in(
            *line,
            [Spec::default().width(ctx.tokens.aside_near), Spec::fill()],
        );
        Label::new(key, ctx.styles.small(Role::Ghost)).draw(ctx, k);
        let style = ctx.styles.small(*role);
        match copies {
            true => copy::Copied::new(value, *role, last).draw(ctx, v),
            false => Label::new(value, style).draw(ctx, v),
        };
    }
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

/// The container's facts, a line each; ports and mounts take a line apiece.
type Fact = (&'static str, String, Role);

/// What the container is doing, then how it is wired: its ports and its mounts, a line apiece.
fn facts(one: &Container, ctx: &Ctx) -> (Vec<Fact>, Vec<Fact>) {
    let mut out = Vec::new();
    let restarts = if one.restarts > 0 {
        Role::Bad
    } else {
        Role::Muted
    };
    out.push(("restarts", one.restarts.to_string(), restarts));
    if let Some(last) = &one.last {
        let when = last
            .at
            .map(|at| format!(" · {} ago", ago(at.age_at(ctx.now))))
            .unwrap_or_default();
        let role = if last.exit == 0 {
            Role::Muted
        } else {
            Role::Bad
        };
        out.push((
            "last ended",
            format!("{} · exit {}{when}", last.reason, last.exit),
            role,
        ));
    }
    let many = |key, values: &[String], out: &mut Vec<_>| {
        for (at, value) in values.iter().enumerate() {
            out.push((if at == 0 { key } else { "" }, value.clone(), Role::Muted));
        }
    };
    for (key, probe) in [("liveness", &one.liveness), ("readiness", &one.readiness)] {
        out.extend(probe.clone().map(|probe| (key, probe, Role::Muted)));
    }
    let mut wiring = Vec::new();
    many("ports", &one.ports, &mut wiring);
    many("mounts", &one.mounts, &mut wiring);
    (out, wiring)
}

/// One resource as a column: the limit above it, the request a tick across, the usage its fill.
fn meter(
    ctx: &mut Ctx,
    rect: Rect,
    (name, (request, limit)): (&str, &(Option<String>, Option<String>)),
    used: Option<f64>,
    write: fn(f64) -> String,
) {
    let row = Spec::default().height(ctx.tokens.row);
    let [title, top, column] = column_in(rect, [row, row, Spec::fill()]);
    Label::new(name, ctx.styles.small(Role::Faint)).draw(ctx, title);
    let read = |one: &Option<String>| one.as_deref().and_then(quantity);
    let Some(gauge) = gauge(read(request), read(limit), used) else {
        Label::new("no request or limit", ctx.styles.small(Role::Ghost)).draw(ctx, top);
        return;
    };
    let wide = ctx.tokens.md * 2.0;
    let bar = column.align((wide, column.h), Align::Center, Align::Start);
    let style = ctx.styles.small(Role::Muted);
    let limit_text = limit.clone().unwrap_or_else(|| "no limit".into());
    let width = ctx.measure(&limit_text, &style);
    Label::new(&limit_text, style)
        .draw(ctx, top.align((width, top.h), Align::Center, Align::Center));
    Panel::default()
        .ground(ctx.styles.deep())
        .border(ctx.styles.line())
        .draw(ctx, bar);
    let role = match gauge.pressure {
        Pressure::Under => Role::Working,
        Pressure::Over => Role::Warn,
        Pressure::Near => Role::Bad,
    };
    let fill = gauge.used.zip(used).map(|(part, used)| (part, write(used)));
    let tick = gauge.request.zip(request.clone());
    marks(ctx, (column, bar), role, fill, tick);
}

/// The usage's fill, labelled on the right where it ends; the request's tick, labelled on the left.
fn marks(
    ctx: &mut Ctx,
    (column, bar): (Rect, Rect),
    role: Role,
    fill: Option<(f64, String)>,
    tick: Option<(f64, String)>,
) {
    let at = |part: f64| bar.bottom() - bar.h * part as f32;
    let (gap, tall) = (ctx.tokens.sm, ctx.tokens.row);
    let beside = |y: f32, left: bool| {
        let (x, w) = match left {
            true => (column.x, bar.x - column.x - gap),
            false => (bar.right() + gap, column.right() - bar.right() - gap),
        };
        Rect {
            x,
            y: y - tall / 2.0,
            w,
            h: tall,
        }
    };
    if let Some((part, used)) = fill {
        let filled = Rect {
            y: at(part),
            h: bar.h * part as f32,
            ..bar
        };
        Panel::default()
            .ground(ctx.styles.color(role))
            .draw(ctx, filled);
        Label::new(&used, ctx.styles.small(role)).draw(ctx, beside(at(part), false));
    }
    if let Some((part, request)) = tick {
        let xs = ctx.tokens.xs;
        let line = Rect {
            y: at(part) - ctx.tokens.hairline,
            h: ctx.tokens.hairline * 2.0,
            x: bar.x - xs,
            w: bar.w + xs * 2.0,
        };
        Panel::default()
            .ground(ctx.styles.color(Role::Text))
            .draw(ctx, line);
        let mut room = beside(at(part), true);
        Label::new(&request, ctx.styles.small(Role::Faint)).right(ctx, &mut room, 0.0);
    }
}
