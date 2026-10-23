//! The band above the rows: on the diff, the whole change and its views; on a file, its path.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Up;
use groove_gfx::{Edges, Rect};
use groove_types::{Config, DiffMode, DiffView, LanguageServer};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Tab, files};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{from_left, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets;

/// The band over the diff or over the open file.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    if ui.session.tab == Tab::Diff {
        let mut room = band.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        totals(ctx, &mut room, app);
        switch(ctx, &mut room, ui.session.view);
        if app.workspace.commit.is_none() {
            from_left(&mut room, ctx.tokens.sm, 0.0);
            modes(ctx, &mut room, app.workspace.mode);
        }
        return;
    }
    named(ctx, band, app, path);
}

/// The file's path from the left, then what stands beside it: unsaved, changed, faults, servers.
fn named(ctx: &mut Ctx, band: Rect, app: &AppState, path: &str) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = band.pad(Edges::across(md, md));
    let style = ctx.styles.code(Role::Muted);
    let counts = super::row::counted(app, path).unwrap_or_default();
    let tally = super::faults::tally(app, path);
    let servers = servers_of(app, path);
    let dirty = app
        .workspace
        .buffer(path)
        .is_some_and(|open| open.new.dirty());
    let after = beside(ctx, (dirty, counts, tally), &servers);
    let wide = ctx.measure(path, &style).min((room.w - after).max(0.0));
    let slot = from_left(&mut room, wide, sm);
    Label::new(path, style).cut_start().draw(ctx, slot);
    let size = ctx.styles.small(Role::Warn).size;
    if dirty {
        let dot = from_left(&mut room, size, sm);
        groove_ui_kit::widgets::icon(ctx, square(dot, size), Mark::Modified, Role::Warn);
    }
    for (count, role, sign) in [(counts.0, Role::Ok, '+'), (counts.1, Role::Bad, '-')] {
        if count > 0 {
            let text = format!("{sign}{count}");
            Label::new(&text, ctx.styles.code(role)).left(ctx, &mut room, sm);
        }
    }
    if let Some(tally) = tally {
        let wide = widgets::faults_room(ctx, tally);
        let mut slot = from_left(&mut room, wide, md);
        widgets::faults(ctx, &mut slot, tally);
    }
    for (name, up) in servers {
        server(ctx, &mut room, &name, up);
    }
}

/// How wide what stands beside the path is.
fn beside(
    ctx: &mut Ctx,
    (dirty, counts, tally): (bool, (u32, u32), Option<(u32, u32)>),
    servers: &[(String, Up)],
) -> f32 {
    let (sm, md, xs) = (ctx.tokens.sm, ctx.tokens.md, ctx.tokens.xs);
    let style = ctx.styles.code(Role::Faint);
    let size = ctx.styles.small(Role::Warn).size;
    let mut wide = if dirty { size + sm } else { 0.0 };
    for (count, sign) in [(counts.0, '+'), (counts.1, '-')] {
        if count > 0 {
            wide += ctx.measure(&format!("{sign}{count}"), &style) + sm;
        }
    }
    if let Some(tally) = tally {
        wide += widgets::faults_room(ctx, tally) + md;
    }
    for (name, _) in servers {
        wide += size + xs + ctx.measure(name, &style) + md;
    }
    wide + sm
}

/// The views after the totals, the current one held.
fn switch(ctx: &mut Ctx, room: &mut Rect, current: DiffView) {
    for view in DiffView::ALL {
        let tab = widgets::Tab::new(view.label(), Target::View(view), view == current).glass();
        tab.left(ctx, room, ctx.tokens.xs);
    }
}

/// What the change is read against, the one in use held.
fn modes(ctx: &mut Ctx, room: &mut Rect, current: DiffMode) {
    for mode in DiffMode::ALL {
        let tab = widgets::Tab::new(mode.label(), Target::Mode(mode), mode == current).glass();
        tab.left(ctx, room, ctx.tokens.xs);
    }
}

/// How many files the change touches, then what it added and removed in all, from the left.
fn totals(ctx: &mut Ctx, room: &mut Rect, app: &AppState) {
    let files = files::changed(app);
    let added: u32 = files.iter().map(|file| file.added).sum();
    let deleted: u32 = files.iter().map(|file| file.deleted).sum();
    let said = match files.len() {
        1 => "1 file".to_string(),
        many => format!("{many} files"),
    };
    let sm = ctx.tokens.sm;
    Label::new(&said, ctx.styles.body(Role::Muted)).left(ctx, room, sm);
    for (count, role, sign) in [(added, Role::Ok, '+'), (deleted, Role::Bad, '-')] {
        if count > 0 {
            let text = format!("{sign}{count}");
            Label::new(&text, ctx.styles.code(role)).left(ctx, room, sm);
        }
    }
    from_left(room, ctx.tokens.sm, 0.0);
}

/// The servers of the open file's language that have started, and where each stands.
fn servers_of(app: &AppState, path: &str) -> Vec<(String, Up)> {
    let (Some(worktree), Some(open)) =
        (app.workspace.worktree.as_ref(), app.workspace.buffer(path))
    else {
        return Vec::new();
    };
    let Some(language) = open.new.document().language() else {
        return Vec::new();
    };
    let config = app.config.config.as_ref();
    let named = config.map_or_else(LanguageServer::defaults, Config::servers);
    let states = app.workspace.servers.states(worktree);
    named
        .into_iter()
        .filter(|one| one.serves(language.id()))
        .filter_map(|one| states.iter().find(|(name, _)| *name == one.name).cloned())
        .collect()
}

/// A server's dot, in the colour of where it stands, then its name, from the left of `room`.
fn server(ctx: &mut Ctx, room: &mut Rect, name: &str, up: Up) {
    let role = match up {
        Up::Ready => Role::Ok,
        Up::Starting => Role::Working,
        Up::Down => Role::Bad,
    };
    let size = ctx.styles.small(role).size;
    let dot = from_left(room, size, ctx.tokens.xs);
    groove_ui_kit::widgets::icon(ctx, square(dot, size), Mark::Modified, role);
    let style = ctx.styles.code(Role::Faint);
    Label::new(name, style).left(ctx, room, ctx.tokens.md);
}
