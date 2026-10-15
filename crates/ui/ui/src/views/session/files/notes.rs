//! The sidebar's notes: open, resolved or all, under the file each stands on.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Note;

use crate::Ui;
use crate::components::{Header, label_role};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::views::session::Showing;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::{Label, ago, row};
use groove_ui_kit::widgets::{Badge, tabs};

/// One row of the list: a file's heading, or a note under it.
pub(super) enum Item {
    /// The file's path, none for the notes on the MR itself, and how many stand under it.
    Head(Option<String>, usize),
    Note(usize),
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let strip = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [strip, body] = column_in(body, [strip, Spec::fill()]);
    filters(ctx, strip, app, ui.session.showing);
    let items = items(app, ui.session.showing);
    if items.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let [line, _] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        return row(ctx, line, ctx.tokens.md, said(ui.session.showing), style);
    }
    let (head, note) = (ctx.tokens.row, ctx.tokens.row + ctx.tokens.line);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    crate::offsets::listed(
        ctx,
        body,
        at,
        &items,
        |item| match item {
            Item::Head(..) => head,
            Item::Note(_) => note,
        },
        |ctx, line, item| match item {
            Item::Head(path, count) => heading(ctx, line, path.as_deref(), *count),
            Item::Note(at) => one(ctx, line, app, *at),
        },
    );
}

/// Open, resolved, all: each counted, the one shown lit.
fn filters(ctx: &mut Ctx, strip: Rect, app: &AppState, showing: Showing) {
    let labels: Vec<String> = Showing::ALL
        .iter()
        .map(|one| {
            let count = app
                .delivery
                .shown
                .iter()
                .filter(|note| one.lists(note.resolved));
            format!("{} · {}", one.label(), count.count())
        })
        .collect();
    let shown: Vec<(&str, Target)> = labels
        .iter()
        .zip(Showing::ALL)
        .map(|(label, one)| (label.as_str(), Target::Showing(one)))
        .collect();
    let at = Showing::ALL
        .iter()
        .position(|one| *one == showing)
        .unwrap_or(0);
    tabs(ctx, strip, &shown, at);
}

/// What an empty list says, for what it shows.
fn said(showing: Showing) -> &'static str {
    match showing {
        Showing::Open => "No open notes",
        Showing::Resolved => "Nothing resolved yet",
        Showing::All => "Right click a line in the diff to add a note",
    }
}

/// The notes shown, each file's under its heading: the change's files in their order, the
/// others by path, the MR's own last.
pub(super) fn items(app: &AppState, showing: Showing) -> Vec<Item> {
    let notes: Vec<(usize, &Note)> = app
        .delivery
        .shown
        .iter()
        .enumerate()
        .filter(|(_, note)| showing.lists(note.resolved))
        .collect();
    let path = |note: &Note| note.anchor.as_ref().map(|anchor| anchor.path.clone());
    let changed: Vec<&str> = app
        .workspace
        .changes
        .files()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    let mut paths: Vec<Option<String>> = notes.iter().map(|(_, note)| path(note)).collect();
    paths.sort_by_key(|one| match one {
        Some(path) => match changed.iter().position(|changed| changed == path) {
            Some(at) => (0, at, path.clone()),
            None => (1, 0, path.clone()),
        },
        None => (2, 0, String::new()),
    });
    paths.dedup();
    let mut out = Vec::new();
    for one in paths {
        let under: Vec<usize> = notes
            .iter()
            .filter(|(_, note)| path(note) == one)
            .map(|(at, _)| *at)
            .collect();
        out.push(Item::Head(one, under.len()));
        out.extend(under.into_iter().map(Item::Note));
    }
    out
}

/// A file's heading: its name, its folder after it, the notes under it at the end.
fn heading(ctx: &mut Ctx, line: Rect, path: Option<&str>, count: usize) {
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let (name, folder) = match path {
        Some(path) => match path.rsplit_once('/') {
            Some((folder, name)) => (name, folder),
            None => (path, ""),
        },
        None => ("on the MR", ""),
    };
    let faint = ctx.styles.small(Role::Faint);
    Label::new(&count.to_string(), faint).right(ctx, &mut room, ctx.tokens.sm);
    Label::new(name, ctx.styles.label(Role::Muted)).left(ctx, &mut room, ctx.tokens.sm);
    Label::new(folder, faint).cut_start().draw(ctx, room);
}

/// One note on two lines: its label and subject; who, from where, which line, and when.
fn one(ctx: &mut Ctx, line: Rect, app: &AppState, at: usize) {
    let Some(note) = app.delivery.shown.get(at) else {
        return;
    };
    hoverable(ctx, line, Target::NoteAt(at));
    let lines = note
        .anchor
        .as_ref()
        .map(|anchor| format!("L{}", anchor.start_line + 1))
        .unwrap_or_default();
    let head = Header::of(note, 0, lines);
    let role = if note.resolved {
        Role::Faint
    } else {
        Role::Text
    };
    let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let half = Spec::default().height(line.h / 2.0);
    let [top, bottom] = column_in(room, [half, Spec::fill()]);
    said_line(ctx, top, note, &head, role);
    who_line(ctx, bottom, note, &head);
}

/// A resolved note's tick, its label and the agent's mark, then what it opens with.
fn said_line(ctx: &mut Ctx, mut room: Rect, note: &Note, head: &Header, role: Role) {
    let gap = ctx.tokens.xs;
    if note.resolved {
        groove_ui_kit::widgets::lead(ctx, &mut room, Mark::Read, Role::Ok);
    }
    if let Some((label, decorations)) = head.label.as_ref() {
        Badge::new(label, label_role(label)).left(ctx, &mut room, gap);
        for decoration in decorations.iter().filter(|one| *one == "agent") {
            Badge::new(decoration, Role::Faint).left(ctx, &mut room, gap);
        }
    }
    Label::new(&subject(note), ctx.styles.body(role)).draw(ctx, room);
}

/// Who wrote it, from where, on which line; how many replies and how long ago at the end.
fn who_line(ctx: &mut Ctx, mut room: Rect, note: &Note, head: &Header) {
    let (gap, faint) = (ctx.tokens.sm, ctx.styles.small(Role::Faint));
    let age = ago(head.at.age_at(ctx.now));
    Label::new(&age, faint).right(ctx, &mut room, gap);
    let replies = match note.replies() {
        0 => String::new(),
        1 => "1 reply".to_string(),
        many => format!("{many} replies"),
    };
    Label::new(&replies, faint).right(ctx, &mut room, gap);
    let initials = ctx.styles.initials(Role::Muted);
    Label::new(&head.initials, initials).left(ctx, &mut room, ctx.tokens.xs);
    for text in [head.author.as_str(), head.source, head.lines.as_str()] {
        Label::new(text, faint).left(ctx, &mut room, gap);
    }
}

/// The first line a note opens with, its label off and its Markdown plain.
fn subject(note: &Note) -> String {
    let body = note.opening().map_or("", |said| said.body.as_str());
    let words = groove_types::labelled(body).map_or(body, |one| one.rest);
    let plain = groove_ui_kit::markdown::plain(words);
    plain.lines().next().unwrap_or_default().to_string()
}
