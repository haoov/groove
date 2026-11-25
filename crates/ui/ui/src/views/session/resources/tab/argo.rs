//! An Argo CD Application's sections: its last sync on its outcome's colour, its conditions, its summary.

use groove_gfx::Rect;
use groove_types::{
    AppCondition, AppPart, Operation, Retry, SyncWindows, SyncedResource, Timestamp,
};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::Label;

use super::super::opened::Section;
use super::describe::{Block, Tally};
use super::sections::Fact;
use crate::ctx::Ctx;

/// The operation, then the conditions where there are some.
pub(super) fn parts(app: &AppPart) -> Vec<(Section, Tally, Vec<Block<'_>>)> {
    let mut out = Vec::new();
    if let Some(operation) = &app.operation {
        let mut held = vec![Block::Operation(operation)];
        if !operation.message.is_empty() {
            held.push(Block::Said(&operation.message));
        }
        held.extend(operation.failed.iter().map(Block::Broke));
        let said = match operation.running() {
            true => "running",
            false => "last sync",
        };
        out.push((Section::Operation, Tally::Note(said.to_string()), held));
    }
    if !app.conditions.is_empty() {
        let count = Tally::Count(app.conditions.len());
        let held = app.conditions.iter().map(Block::AppCondition).collect();
        out.push((Section::Conditions, count, held));
    }
    out
}

/// The colour the operation stands on: red when it failed, blue while it runs.
pub(super) fn tint(app: &AppPart) -> Option<Role> {
    let operation = app.operation.as_ref()?;
    match () {
        _ if operation.failed() => Some(Role::Bad),
        _ if operation.running() => Some(Role::Working),
        _ => None,
    }
}

/// A row of the operation.
pub(super) fn operated(ctx: &mut Ctx, rect: Rect, block: &Block) {
    match block {
        Block::Operation(one) => operation(ctx, rect, one, ctx.now),
        Block::Said(message) => said(ctx, rect, message),
        Block::Broke(one) => broke(ctx, rect, one),
        _ => {}
    }
}

/// A row of the operation, on its colour.
pub(super) fn ground(ctx: &mut Ctx, line: Rect, role: Role) {
    Panel::default()
        .ground(ctx.styles.tint(role))
        .draw(ctx, line);
}

/// `✗ Sync failed to a1b2c3d · started 4m ago by auto-sync · 38s · retry 2`.
pub(super) fn operation(ctx: &mut Ctx, rect: Rect, operation: &Operation, now: Timestamp) {
    let (said, role) = match operation.phase.as_str() {
        "Failed" => ("✗ Sync failed", Role::Bad),
        "Error" => ("✗ Sync error", Role::Bad),
        "Running" => ("Syncing", Role::Working),
        "Terminating" => ("Stopping", Role::Working),
        _ => ("✓ Synced", Role::Ok),
    };
    let mut room = rect;
    if operation.running() {
        room = turning(ctx, room, role);
    }
    Label::new(said, ctx.styles.body(role)).left(ctx, &mut room, ctx.tokens.sm);
    let mut about = Vec::new();
    if let Some(revision) = &operation.revision {
        about.push(format!("to {}", groove_types::short_sha(revision)));
    }
    if let Some(started) = operation.started {
        let by = match operation.by.is_empty() {
            true => String::new(),
            false => format!(" by {}", operation.by),
        };
        about.push(format!(
            "started {} ago{by}",
            groove_types::human_age(started.age_at(now).as_secs())
        ));
    }
    let ended = operation.finished.or(operation.running().then_some(now));
    if let (Some(started), Some(ended)) = (operation.started, ended) {
        about.push(groove_types::human_age(started.age_at(ended).as_secs()));
    }
    if operation.retries > 0 {
        about.push(format!("retry {}", operation.retries));
    }
    Label::new(&about.join(" · "), ctx.styles.small(Role::Faint)).draw(ctx, room);
}

/// The busy mark turning at the left of `room`; returns the room after it.
pub(super) fn turning(ctx: &mut Ctx, room: Rect, role: Role) -> Rect {
    let size = Spec::default().width(ctx.tokens.small);
    let gap = Spec::default().width(ctx.tokens.xs);
    let [mark, _, rest] = row_in(room, [size, gap, Spec::fill()]);
    let tall = Spec::default().height(ctx.tokens.small);
    let [_, mark, _] = column_in(mark, [Spec::fill(), tall, Spec::fill()]);
    groove_ui_kit::widgets::busy(ctx, mark, role);
    rest
}

pub(super) fn said(ctx: &mut Ctx, rect: Rect, message: &str) {
    Label::new(message, ctx.styles.body(Role::Text)).draw(ctx, rect);
}

/// A resource the sync could not apply: its kind and name, then why.
pub(super) fn broke(ctx: &mut Ctx, rect: Rect, one: &SyncedResource) {
    let mark = Spec::default().width(ctx.tokens.lg);
    let kind = Spec::default().width(ctx.tokens.aside_near);
    let name = ctx.tokens.aside_mid * ctx.tokens.text / ctx.tokens.small;
    let name = Spec::default().width(name);
    let [at, of, named, why] = row_in(rect, [mark, kind, name, Spec::fill()]);
    Label::new("✗", ctx.styles.small(Role::Bad)).draw(ctx, at);
    let tag = match one.hook {
        true => format!("{} · hook", one.kind.to_lowercase()),
        false => one.kind.to_lowercase(),
    };
    Label::new(&tag, ctx.styles.small(Role::Ghost)).draw(ctx, of);
    Label::new(&one.name, ctx.styles.body(Role::Text)).draw(ctx, named);
    Label::new(&one.message, ctx.styles.body(Role::Bad)).draw(ctx, why);
}

/// A condition: a warning in yellow, an error in red, then what it says and since when.
pub(super) fn condition(ctx: &mut Ctx, rect: Rect, one: &AppCondition, now: Timestamp) {
    let role = match one.kind.ends_with("Warning") {
        true => Role::Warn,
        false => Role::Bad,
    };
    let mut room = rect;
    Label::new(&one.kind, ctx.styles.body(role)).left(ctx, &mut room, ctx.tokens.md);
    if let Some(at) = one.at {
        let since = format!("{} ago", groove_types::human_age(at.age_at(now).as_secs()));
        Label::new(&since, ctx.styles.small(Role::Ghost)).right(ctx, &mut room, ctx.tokens.md);
    }
    Label::new(&one.message, ctx.styles.body(Role::Text)).draw(ctx, room);
}

/// Where it syncs to and on which context, once the kubeconfig is `read`, and how.
pub(super) fn facts(app: &AppPart, context: Option<String>, read: bool) -> Vec<Fact> {
    let to = &app.destination;
    let cluster = to.name.clone().or(to.server.clone()).unwrap_or_default();
    let namespace = to.namespace.clone().unwrap_or_default();
    let context = match context {
        Some(name) => Some(Fact::new("context", name, Role::Text)),
        None if read => Some(Fact::new(
            "context",
            "none added in Groove".into(),
            Role::Warn,
        )),
        None => None,
    };
    let mut out = vec![
        Fact::new("project", app.project.clone(), Role::Text),
        Fact::new(
            "destination",
            format!("{cluster} · {namespace}"),
            Role::Text,
        ),
    ];
    out.extend(context);
    out
}

/// How it syncs: by itself or by hand, the options every sync applies, and how a failure retries.
pub(super) fn policy<'a>(
    app: &'a AppPart,
    windows: Option<&Result<SyncWindows, String>>,
) -> (Section, Tally, Vec<Block<'a>>) {
    let on = |set: bool| if set { "on" } else { "off" };
    let automated = match app.automated {
        Some((prune, heal)) => {
            let said = format!("on · prune {} · self-heal {}", on(prune), on(heal));
            Fact::new("automated", said, Role::Ok)
        }
        None => Fact::new("automated", "off: it syncs when asked".into(), Role::Muted),
    };
    let options = match app.options.is_empty() {
        true => Fact::new("options", "none".into(), Role::Muted),
        false => Fact::new("options", app.options.join(" · "), Role::Text),
    };
    let retry = match &app.retry {
        Some(retry) => Fact::new("retry", retried(retry), Role::Text),
        None => Fact::new("retry", "none".into(), Role::Muted),
    };
    let facts = vec![automated, options, retry, windowed(windows)];
    (
        Section::Policy,
        Tally::Count(0),
        vec![Block::Summary(facts)],
    )
}

/// `shut · deny 0 22 * * * for 8h Europe/Paris, by hand passes, active`.
fn windowed(windows: Option<&Result<SyncWindows, String>>) -> Fact {
    let (said, role) = match windows {
        None => ("reading".to_string(), Role::Faint),
        Some(Err(why)) => (why.clone(), Role::Bad),
        Some(Ok(read)) if read.assigned.is_empty() => ("none".to_string(), Role::Muted),
        Some(Ok(read)) => {
            let each = read.assigned.iter().map(|one| {
                let mut said = format!("{} {} for {}", one.kind, one.schedule, one.duration);
                if !one.time_zone.is_empty() {
                    said.push_str(&format!(" {}", one.time_zone));
                }
                if one.manual {
                    said.push_str(", by hand passes");
                }
                if read.active.contains(one) {
                    said.push_str(", active");
                }
                said
            });
            let each: Vec<String> = each.collect();
            match read.can_sync {
                true => (format!("open · {}", each.join(" · ")), Role::Ok),
                false => (format!("shut · {}", each.join(" · ")), Role::Warn),
            }
        }
    };
    Fact::new("windows", said, role)
}

/// `5 times · 5s backoff ×2 up to 3m`.
fn retried(retry: &Retry) -> String {
    let mut said = vec![match retry.limit {
        limit if limit < 0 => "without end".to_string(),
        1 => "once".to_string(),
        limit => format!("{limit} times"),
    }];
    if let Some(backoff) = &retry.backoff {
        let factor = retry
            .factor
            .map(|one| format!(" ×{one}"))
            .unwrap_or_default();
        let max = retry
            .max
            .as_ref()
            .map(|one| format!(" up to {one}"))
            .unwrap_or_default();
        said.push(format!("{backoff} backoff{factor}{max}"));
    }
    said.join(" · ")
}
