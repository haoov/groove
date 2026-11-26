//! The Overview's sidebar: the task's fields, or the review's threads and pipeline; then the
//! clusters the session holds and the children it spawned.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{MrThread, SessionKind};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::from_top;
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{Heading, cap, cap_right, flat_row, panel_head, solid_pane};

use super::super::{children, clusters, task};
use crate::ctx::Ctx;
use crate::hit::Target;

/// The sidebar in `rect`, its head folding it.
pub fn draw(ctx: &mut Ctx, app: &AppState, open: &Open, (rect, shown): (Rect, bool)) {
    solid_pane(ctx, rect);
    let mut room = rect;
    let top = from_top(&mut room, ctx.tokens.bar, 0.0);
    panel_head(ctx, top, !shown);
    let row = flat_row(ctx, top, Target::AsideFold, false);
    let kind = match open.session.kind {
        SessionKind::Review { .. } => "review",
        SessionKind::Explorer => "explorer",
        _ => "task",
    };
    Heading::new(kind).fold(shown).draw(ctx, row);
    if !shown {
        return;
    }
    let mut room = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    match open.session.kind {
        SessionKind::Review { .. } => reviewed(ctx, app, open, &mut room),
        _ => fields(ctx, app, open, &mut room),
    }
    held(ctx, app, open, &mut room);
}

/// Each field the task has a value for, its name over the value's column.
fn fields(ctx: &mut Ctx, app: &AppState, open: &Open, room: &mut Rect) {
    let Some(one) = app.task.worked(&open.session) else {
        return;
    };
    let mut said = vec![("source", one.short_id.clone(), Role::Text)];
    said.extend(task::said(one, open.state.maintenance));
    for (name, value, role) in said {
        let mut line = from_top(room, ctx.tokens.row, 0.0);
        let mut first = groove_ui_kit::shape::from_left(&mut line, ctx.tokens.aside_near, 0.0);
        cap(ctx, &mut first, name, Role::Ghost);
        Label::new(&value, ctx.styles.body(role)).draw(ctx, line);
    }
}

/// The review's MR threads, open ones first, then its pipeline.
fn reviewed(ctx: &mut Ctx, app: &AppState, open: &Open, room: &mut Rect) {
    let read = open
        .worktrees
        .iter()
        .find_map(|one| app.delivery.held(&one.id)?.read.as_ref());
    let Some(read) = read else {
        return;
    };
    let mut threads: Vec<&MrThread> = read.threads.iter().collect();
    threads.sort_by_key(|one| resolved(one));
    heading(ctx, room, &format!("mr threads · {}", threads.len()));
    for one in threads {
        thread(ctx, room, one);
    }
    if let Some(ci) = read.ci.as_ref() {
        heading(ctx, room, "pipeline");
        let mut line = from_top(room, ctx.tokens.row, 0.0);
        if let Some(at) = ci.finished_at {
            let said = ago(at.age_at(ctx.now));
            Label::new(&said, ctx.styles.log(Role::Ghost)).right(ctx, &mut line, ctx.tokens.sm);
        }
        let role = super::repo::ci_role(ci.state);
        Label::new(ci.state.label(), ctx.styles.body(role)).draw(ctx, line);
    }
}

fn resolved(thread: &MrThread) -> bool {
    thread.notes.first().is_some_and(|one| one.resolved)
}

/// One thread: who opened it and its first line, open in the ask's colour.
fn thread(ctx: &mut Ctx, room: &mut Rect, one: &MrThread) {
    let Some(first) = one.notes.first() else {
        return;
    };
    let mut line = from_top(room, ctx.tokens.row, 0.0);
    let (word, role) = match resolved(one) {
        true => ("resolved", Role::Ghost),
        false => ("open", Role::Attention),
    };
    cap_right(ctx, &mut line, word, role);
    let opening = first.body.lines().next().unwrap_or_default();
    let said = format!("{} · {opening}", first.author);
    Label::new(&said, ctx.styles.small(Role::Muted)).draw(ctx, line);
}

/// The clusters the session holds, each context in its hue, then the children it spawned.
fn held(ctx: &mut Ctx, app: &AppState, open: &Open, room: &mut Rect) {
    let contexts = crate::views::session::resources::contexts(open);
    if !contexts.is_empty() {
        heading(ctx, room, &format!("clusters · {}", contexts.len()));
    }
    for context in contexts {
        let hue = app.config.cluster(context).map(|one| one.hue);
        let on = open.clusters.iter().filter(|one| one.context == context);
        let names: Vec<Option<&str>> = on.map(|one| one.namespace.as_deref()).collect();
        let line = from_top(room, ctx.tokens.row, 0.0);
        clusters::draw(ctx, line, (context, hue), &names);
    }
    if !open.children.is_empty() {
        heading(ctx, room, &format!("children · {}", open.children.len()));
    }
    for one in &open.children {
        let line = from_top(room, ctx.tokens.row, 0.0);
        children::child(ctx, app, line, one);
    }
}

fn heading(ctx: &mut Ctx, room: &mut Rect, word: &str) {
    let line = from_top(room, ctx.tokens.row + ctx.tokens.sm, ctx.tokens.xs);
    let mut line = line.pad(Edges {
        top: ctx.tokens.sm,
        ..Edges::all(0.0)
    });
    cap(ctx, &mut line, word, Role::Ghost);
}
