//! The band above the rows: on the diff, the whole change and its views; on a file, its path.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Up;
use groove_gfx::Rect;
use groove_types::{Config, DiffMode, DiffView, LanguageServer};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{from_left, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets;

/// The file's path from the very left of `room`, then what stands beside it.
pub(super) fn written(ctx: &mut Ctx, mut room: Rect, app: &AppState, path: &str) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
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

/// The views, then what the change is read against unless it is read-only, at the right of `room`.
pub(super) fn switches(ctx: &mut Ctx, room: &mut Rect, app: &AppState, ui: &Ui) {
    let (right, gap) = (widgets::Side::Right, ctx.tokens.sm);
    if !app.workspace.readonly() {
        let mode = app.workspace.mode;
        let tabs =
            DiffMode::ALL.map(|one| widgets::Tab::new(one.label(), Target::Mode(one), one == mode));
        widgets::tab_group(ctx, room, tabs.into(), (right, gap));
    }
    let view = ui.session.view;
    let tabs =
        DiffView::ALL.map(|one| widgets::Tab::new(one.label(), Target::View(one), one == view));
    widgets::tab_group(ctx, room, tabs.into(), (right, gap));
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
