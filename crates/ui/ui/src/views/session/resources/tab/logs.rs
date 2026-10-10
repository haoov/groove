//! A pod's logs: which containers and run they read, from where, then the lines in the editor.

use groove_controllers::AppState;
use groove_controllers::cluster_service::{CAP, Log, Streaming};
use groove_gfx::{Edges, Rect};
use groove_types::{LogSource, PodPart};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Tab, Text};

use super::super::opened::Opened;
use super::super::{RANGES, log_key};
use crate::ctx::Ctx;
use crate::hit::Target;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, (ui, tab): (&crate::Ui, &Opened)) {
    let Some(pod) = tab.link.read(app).and_then(|one| one.pod.as_deref()) else {
        return;
    };
    let bar = Spec::default().height(ctx.tokens.row + ctx.tokens.sm * 2.0);
    let foot = Spec::default().height(ctx.tokens.row);
    let [top, lines, bottom] = column_in(body, [bar, Spec::fill(), foot]);
    let log = log_key(app, tab).and_then(|key| app.cluster.store.logs.get(&key));
    toolbar(ctx, top, (tab, pod), log);
    match log {
        Some(log) if log.lines() > 0 => crate::views::session::diff::editor(ctx, app, ui, lines),
        _ => waiting(ctx, lines, log),
    }
    footer(ctx, bottom, (tab, log));
}

/// The containers, the run, the range; then the clocks and whether the lines follow.
fn toolbar(ctx: &mut Ctx, line: Rect, (tab, pod): (&Opened, &PodPart), log: Option<&Log>) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let chosen = tab
        .logs
        .source
        .clone()
        .or_else(|| super::super::first_container(pod).map(LogSource::Container));
    for one in &pod.containers {
        let label = match one.init {
            true => format!("init · {}", one.name),
            false => one.name.clone(),
        };
        let source = LogSource::Container(one.name.clone());
        let on = chosen.as_ref() == Some(&source);
        small(Tab::new(&label, Target::LogSource(source), on)).left(ctx, &mut room, 0.0);
    }
    if pod.containers.len() > 1 {
        let on = chosen == Some(LogSource::All);
        let all = Tab::new("all containers", Target::LogSource(LogSource::All), on);
        small(all).left(ctx, &mut room, ctx.tokens.md);
    }
    let restarted = pod.containers.iter().any(|one| one.restarts > 0);
    small(Tab::new(
        "current",
        Target::LogPrevious(false),
        !tab.logs.previous,
    ))
    .left(ctx, &mut room, 0.0);
    match restarted {
        true => {
            let previous = Tab::new("previous", Target::LogPrevious(true), tab.logs.previous);
            small(previous).left(ctx, &mut room, ctx.tokens.md);
        }
        false => {
            let style = ctx.styles.small(Role::Ghost);
            Label::new("previous", style).left(ctx, &mut room, ctx.tokens.md);
        }
    }
    let range = RANGES
        .iter()
        .zip(groove_types::LogRange::OFFERED)
        .find(|(_, one)| *one == tab.logs.range)
        .map_or("", |(label, _)| *label);
    let picker = format!("{range} ▾");
    small(Tab::new(&picker, Target::LogRange, false)).left(ctx, &mut room, 0.0);
    stands(ctx, &mut room, (tab, log));
}

/// A control of the bar: small text, no padding.
fn small(tab: Tab<'_, Target>) -> Tab<'_, Target> {
    tab.text(Text::Small).tight()
}

/// From the right: how the stream stands, where it does not simply stream.
fn stands(ctx: &mut Ctx, room: &mut Rect, (tab, log): (&Opened, Option<&Log>)) {
    let (said, role) = match log.map(|one| &one.state) {
        Some(Streaming::Failed(_)) => ("not read", Role::Bad),
        Some(Streaming::Ended) if tab.logs.previous => ("read to its end", Role::Faint),
        Some(Streaming::Ended) => ("container stopped", Role::Faint),
        _ => return,
    };
    Label::new(said, ctx.styles.small(role)).right(ctx, room, ctx.tokens.lg);
}

/// Why no line shows yet: the stream opening, refused, or empty.
fn waiting(ctx: &mut Ctx, body: Rect, log: Option<&Log>) {
    let (said, role) = match log.map(|one| &one.state) {
        Some(Streaming::Failed(why)) => (why.clone(), Role::Bad),
        Some(Streaming::Open | Streaming::Ended) => ("no line yet".to_string(), Role::Faint),
        _ => ("opening the logs…".to_string(), Role::Working),
    };
    let line = body.pad(Edges::all(ctx.tokens.md));
    let [line, _] = column_in(line, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(&said, ctx.styles.small(role)).draw(ctx, line);
}

/// How much the stream holds against what it may; on the right, whether the view follows its end.
fn footer(ctx: &mut Ctx, line: Rect, (tab, log): (&Opened, Option<&Log>)) {
    groove_ui_kit::shape::top_rule(ctx, line, ctx.styles.line());
    let Some(log) = log else {
        return;
    };
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let below = log
        .buffer()
        .lines()
        .saturating_sub(ctx.app.hits.shown().end);
    match tab.logs.following {
        true => {
            let style = ctx.styles.small(Role::Ok);
            Label::new("following", style).right(ctx, &mut room, ctx.tokens.lg);
        }
        false => {
            let said = format!("↓ {below} lines below · follow");
            let follow = Tab::new(&said, Target::LogFollow, false).quiet(Role::Warn);
            small(follow).right(ctx, &mut room, ctx.tokens.lg);
        }
    }
    let style = ctx.styles.small(Role::Ghost);
    let lines = format!("{} lines", log.lines());
    Label::new(&lines, style).left(ctx, &mut room, ctx.tokens.lg);
    let held = format!(
        "{} of {}",
        groove_types::bytes(log.bytes() as f64),
        groove_types::bytes(CAP as f64)
    );
    Label::new(&held, style).left(ctx, &mut room, ctx.tokens.lg);
    if let Streaming::Failed(why) = &log.state {
        Label::new(why, ctx.styles.small(Role::Bad)).draw(ctx, room);
    }
}
