//! An Application's sync as a table: each source under its band, then the revision it targets
//! and the one it last synced, with the commit's author, age and message.

use std::collections::BTreeMap;

use groove_controllers::cluster_service::Revised;
use groove_controllers::{AppState, Command, cluster};
use groove_gfx::Rect;
use groove_types::{AppPart, AppSource, RevisionMeta, Timestamp};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{Cell, Column, Shown, Table, Width};

use super::super::opened::{Link, Opened, Opens, Section};
use super::describe::{Block, Tally};
use super::grouped::{self, Columned};
use super::{copy, source_band};
use crate::ctx::Ctx;
use crate::hit::Target;

type Parted<'a> = (Section, Tally, Vec<Block<'a>>);

/// A line of the table: a source on its band, which folds, or one of its revisions.
pub(super) enum Line {
    Source(Box<source_band::Source>),
    Revision(Box<Revision>),
}

/// A revision a source stands at and what Argo CD said of it; `behind` once the target moved on.
pub(super) struct Revision {
    tag: &'static str,
    role: Role,
    behind: bool,
    revision: String,
    opens: Option<Opens>,
    read: Read,
}

enum Read {
    /// A chart version, or a commit not read yet.
    None,
    Meta(RevisionMeta),
    Failed(String),
}

/// What the section's heading says in place of its name: `out of sync · 3 resources · target moved`.
fn said(part: &AppPart) -> Tally {
    let state = match part.sync.as_str() {
        "OutOfSync" => "out of sync".to_string(),
        "" => "sync unknown".to_string(),
        other => other.to_lowercase(),
    };
    let mut said = vec![state];
    let differ = part
        .resources
        .iter()
        .filter(|one| one.out_of_sync())
        .count();
    if differ > 0 {
        let s = if differ == 1 { "" } else { "s" };
        said.push(format!("{differ} resource{s}"));
    }
    let moved = |one: &AppSource| one.compared.is_some() && one.compared != one.synced;
    if part.sources.iter().any(moved) {
        said.push("target moved".into());
    }
    let said = said.join(" · ");
    match part.sync.as_str() {
        "OutOfSync" => Tally::Warned(said),
        _ => Tally::Note(said),
    }
}

/// The section: its lines as blocks, the first under the table's header.
pub(super) fn section<'a>(app: &AppState, tab: &Opened, part: &AppPart) -> Parted<'a> {
    let lines = lines(app, tab, part).into_iter().enumerate();
    let blocks = lines.map(|(at, line)| Block::Revision(line, at == 0));
    (Section::Sync, said(part), blocks.collect())
}

fn lines(app: &AppState, tab: &Opened, part: &AppPart) -> Vec<Line> {
    let mut out = Vec::new();
    for (at, one) in part.sources.iter().enumerate() {
        let shut = tab.folded(&key(at), false);
        out.push(Line::Source(Box::new(source_band::source(one, at, shut))));
        if shut {
            continue;
        }
        let stands = stands(one);
        let behind = stands.len() > 1;
        for (at_row, (tag, role, revision)) in stands.into_iter().enumerate() {
            let revised = revised(&tab.link, part, at, revision);
            let read = match app.cluster.store.revisions.get(&revised) {
                Some(Ok(meta)) if one.git() => Read::Meta(meta.clone()),
                Some(Err(why)) if one.git() => Read::Failed(why.clone()),
                _ => Read::None,
            };
            let opens = opens(one, revision);
            let revision = revision.to_string();
            out.push(Line::Revision(Box::new(Revision {
                tag,
                role,
                behind: behind && at_row > 0,
                revision,
                opens,
                read,
            })));
        }
    }
    out
}

/// The commits the sources stand at, each read once from Argo CD.
pub(super) fn asks(app: &AppState, link: &Link, part: &AppPart) -> Vec<Command> {
    let mut out = Vec::new();
    for (at, one) in part.sources.iter().enumerate().filter(|(_, one)| one.git()) {
        for (_, _, revision) in stands(one) {
            let revised = revised(link, part, at, revision);
            if groove_types::is_sha(revision) && !app.cluster.store.revisions.asked(&revised) {
                let revised = Box::new(revised);
                out.push(Command::Cluster(cluster::Command::ArgoRevision { revised }));
            }
        }
    }
    out
}

/// The target when it moved past the last sync, then the last sync; one line when they agree.
fn stands(one: &AppSource) -> Vec<(&'static str, Role, &str)> {
    let synced = one.synced.as_deref();
    let compared = one.compared.as_deref();
    match (compared, synced) {
        (Some(to), Some(at)) if to != at => {
            vec![("TARGET", Role::Warn, to), ("SYNCED", Role::Ok, at)]
        }
        (_, Some(at)) => vec![("SYNCED", Role::Ok, at)],
        (Some(to), None) => vec![("TARGET · NEVER SYNCED", Role::Warn, to)],
        (None, None) => Vec::new(),
    }
}

fn revised(link: &Link, part: &AppPart, at: usize, revision: &str) -> Revised {
    let namespace = link.namespace.clone().unwrap_or_default();
    let app = (link.context.clone(), namespace, link.name.clone());
    let source = (part.sources.len() > 1).then_some(at);
    (app, source, revision.to_string())
}

pub(super) fn key(at: usize) -> String {
    format!("sync/source-{at}")
}

/// The columns a revision fills: its tag and sha, who and when, then the message across the rest.
const COLUMNS: [Columned; 4] = [
    ("REVISION", "a1b2c3d4"),
    ("AUTHOR", "Jean-Baptiste Doe"),
    ("WHEN", "23h ago"),
    ("MESSAGE", ""),
];

/// One line, under the table's header when `header`; the columns as the user dragged them.
pub(super) fn draw(
    ctx: &mut Ctx,
    rect: Rect,
    (line, header): (&Line, bool),
    (last, dragged): (Option<&str>, &BTreeMap<String, f32>),
) {
    let caret = Cell::fold(true, Role::Ghost);
    let mut columns = vec![
        Column::plain("", Width::FitCell(&caret)),
        Column::plain("", Width::Fit("TARGET · NEVER SYNCED")),
    ];
    let end = COLUMNS.len() - 1;
    let dragging = COLUMNS.iter().enumerate();
    columns.extend(dragging.map(|(at, one)| grouped::column(one, at == end, dragged)));
    let table = grouped::single(ctx, &columns, header);
    match line {
        Line::Source(one) => source_band::draw(ctx, (table, rect), one, last),
        Line::Revision(one) => revision(ctx, (table, rect), one, last),
    }
}

/// A revision's row: its tag, its short sha that links to its diff, and what Argo CD said of it.
fn revision(
    ctx: &mut Ctx,
    (table, rect): (Table<'_, Target>, Rect),
    one: &Revision,
    last: Option<&str>,
) {
    let (author, when, message) = read(ctx.now, &one.read, one.behind);
    let cells = vec![
        Cell::small("", Role::Ghost),
        Cell::small(one.tag, one.role).badge(),
        Cell::small("", Role::Text),
        Cell::body(author, Role::Muted),
        Cell::small(when, Role::Ghost),
        message,
    ];
    let laid = table.one(ctx, rect, Shown::row(cells));
    let shown = groove_types::short_sha(&one.revision);
    let role = match one.behind {
        true => Role::Muted,
        false => one.role,
    };
    let copied = copy::Copied::new(&shown, role, last).copies(&one.revision);
    copied.opens(one.opens.clone()).draw(ctx, laid.cell(0, 2));
}

/// The author, the age and the message cell of what Argo CD said.
fn read(now: Timestamp, read: &Read, behind: bool) -> (String, String, Cell<'_>) {
    match read {
        Read::None => (String::new(), String::new(), Cell::body("", Role::Text)),
        Read::Failed(why) => (
            String::new(),
            String::new(),
            Cell::small(why.as_str(), Role::Bad),
        ),
        Read::Meta(meta) => {
            let ago = meta.date.map(|at| {
                let age = groove_types::human_age(at.age_at(now).as_secs());
                format!("{age} ago")
            });
            let role = if behind { Role::Muted } else { Role::Text };
            let message = Cell::body(meta.message.as_str(), role);
            (meta.author.clone(), ago.unwrap_or_default(), message)
        }
    }
}

/// A git commit opens its diff in the session; a chart version opens nothing.
fn opens(one: &AppSource, revision: &str) -> Option<Opens> {
    let commit = one.git() && groove_types::is_sha(revision);
    commit.then(|| Opens::Revision {
        repo: one.repo.clone(),
        target: one.target.clone(),
        sha: revision.to_string(),
    })
}
