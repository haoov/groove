//! An Argo CD Application's sections: its last sync on its outcome's colour, its conditions, its summary.

use groove_gfx::Rect;
use groove_types::{AppCondition, AppPart, AppSource, Operation, SyncedResource, Timestamp};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, column_of, row_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::Label;

use super::super::opened::{Opens, Section};
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

/// A row of the operation, on its colour from the panel's edge to the other.
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
    Label::new(said, ctx.styles.small(role)).left(ctx, &mut room, ctx.tokens.sm);
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
    Label::new(message, ctx.styles.small(Role::Text)).draw(ctx, rect);
}

/// A resource the sync could not apply: its kind and name, then why.
pub(super) fn broke(ctx: &mut Ctx, rect: Rect, one: &SyncedResource) {
    let mark = Spec::default().width(ctx.tokens.lg);
    let kind = Spec::default().width(ctx.tokens.aside_near);
    let name = Spec::default().width(ctx.tokens.aside_mid);
    let [at, of, named, why] = row_in(rect, [mark, kind, name, Spec::fill()]);
    Label::new("✗", ctx.styles.small(Role::Bad)).draw(ctx, at);
    let tag = match one.hook {
        true => format!("{} · hook", one.kind.to_lowercase()),
        false => one.kind.to_lowercase(),
    };
    Label::new(&tag, ctx.styles.small(Role::Ghost)).draw(ctx, of);
    Label::new(&one.name, ctx.styles.small(Role::Text)).draw(ctx, named);
    Label::new(&one.message, ctx.styles.small(Role::Bad)).draw(ctx, why);
}

/// A condition: a warning in yellow, an error in red, then what it says and since when.
pub(super) fn condition(ctx: &mut Ctx, rect: Rect, one: &AppCondition, now: Timestamp) {
    let role = match one.kind.ends_with("Warning") {
        true => Role::Warn,
        false => Role::Bad,
    };
    let mut room = rect;
    Label::new(&one.kind, ctx.styles.small(role)).left(ctx, &mut room, ctx.tokens.md);
    if let Some(at) = one.at {
        let since = format!("{} ago", groove_types::human_age(at.age_at(now).as_secs()));
        Label::new(&since, ctx.styles.small(Role::Ghost)).right(ctx, &mut room, ctx.tokens.md);
    }
    Label::new(&one.message, ctx.styles.small(Role::Text)).draw(ctx, room);
}

/// Where it syncs to and on which context, once the kubeconfig is `read`, and how.
pub(super) fn facts(app: &AppPart, context: Option<String>, read: bool) -> Vec<Fact> {
    let to = &app.destination;
    let cluster = to.name.clone().or(to.server.clone()).unwrap_or_default();
    let namespace = to.namespace.clone().unwrap_or_default();
    let said = groove_types::sync_policy(app.automated);
    let role = match app.automated.is_some() {
        true => Role::Ok,
        false => Role::Muted,
    };
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
    out.push(Fact::new("auto-sync", said, role));
    out
}

/// What it syncs from: a row a source, each with the revision it synced to.
pub(super) fn sourced(app: &AppPart) -> (Section, Tally, Vec<Block<'_>>) {
    let count = Tally::Count(app.sources.len());
    (
        Section::Sources,
        count,
        app.sources.iter().map(Block::Source).collect(),
    )
}

/// How many lines a source takes: what it is, where it lives, and its value files when it has some.
pub(super) fn source_rows(one: &AppSource) -> usize {
    2 + usize::from(!one.values.is_empty())
}

/// A source: its name, what it follows and what it synced to; under it, its address and value files.
pub(super) fn source(ctx: &mut Ctx, rect: Rect, one: &AppSource, last: Option<&str>) {
    let row = Spec::default().height(ctx.tokens.row);
    let lines = column_of(rect, &vec![row; source_rows(one)]);
    let tag = Spec::default().width(ctx.tokens.lg * 2.0);
    let [kind, head] = row_in(lines[0], [tag, Spec::fill()]);
    let is_chart = one.chart.is_some();
    let said = if is_chart { "chart" } else { "git" };
    Label::new(said, ctx.styles.small(Role::Ghost)).draw(ctx, kind);
    let mut room = head;
    let gap = ctx.tokens.md;
    Label::new(&named(one), ctx.styles.small(Role::Text)).left(ctx, &mut room, gap);
    Label::new(&one.target, ctx.styles.small(Role::Muted)).left(ctx, &mut room, gap);
    if let Some(revision) = &one.synced {
        Label::new("synced", ctx.styles.small(Role::Ghost)).left(ctx, &mut room, ctx.tokens.sm);
        synced(ctx, &mut room, (one, revision), last);
    }
    let [_, under] = row_in(lines[1], [tag, Spec::fill()]);
    let mut room = under;
    let bare = bare(&one.repo);
    let opens = (!is_chart).then(|| Opens::Repo {
        repo: one.repo.clone(),
        target: one.target.clone(),
    });
    let role = if is_chart { Role::Faint } else { Role::Accent };
    let address = super::copy::Copied::new(&bare, role, last).opens(opens);
    address.left(ctx, &mut room, gap);
    if let Some(name) = &one.reference {
        let said = format!("· referenced as ${name}");
        Label::new(&said, ctx.styles.small(Role::Ghost)).draw(ctx, room);
    }
    if let Some(line) = lines.get(2) {
        let [_, values] = row_in(*line, [tag, Spec::fill()]);
        let said = format!("values  {}", one.values.join(", "));
        Label::new(&said, ctx.styles.small(Role::Faint)).draw(ctx, values);
    }
}

/// The revision a source synced to, short; a git commit links to its diff.
fn synced(ctx: &mut Ctx, room: &mut Rect, (one, revision): (&AppSource, &str), last: Option<&str>) {
    let opens = (one.chart.is_none() && groove_types::is_sha(revision)).then(|| Opens::Revision {
        repo: one.repo.clone(),
        target: one.target.clone(),
        sha: revision.to_string(),
    });
    let role = if opens.is_some() {
        Role::Accent
    } else {
        Role::Text
    };
    let shown = groove_types::short_sha(revision);
    let synced = super::copy::Copied::new(&shown, role, last).copies(revision);
    synced.opens(opens).left(ctx, room, ctx.tokens.md);
}

/// What a source is called: its chart, or its repository's name and the path in it.
fn named(one: &AppSource) -> String {
    if let Some(chart) = &one.chart {
        return chart.clone();
    }
    let repo = bare(&one.repo);
    let name = repo.rsplit('/').next().unwrap_or(&repo).to_string();
    match &one.path {
        Some(path) => format!("{name} · {path}"),
        None => name,
    }
}

/// A repository's address without its scheme, its credentials and `.git`.
fn bare(repo: &str) -> String {
    let path = repo.split_once("://").map_or(repo, |(_, rest)| rest);
    let host = path.find('/').unwrap_or(path.len());
    let path = match path[..host].rfind('@') {
        Some(at) => &path[at + 1..],
        None => path,
    };
    path.trim_end_matches(".git").to_string()
}
