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
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::{from_left, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{self, changes};

/// The band over the diff or over the open file.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    let mut room = band;
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
    if let Some(counts) = super::row::counted(app, path) {
        changes(ctx, &mut room, counts, |ctx, role| ctx.styles.code(role));
    }
    for (name, up) in servers_of(app, path).into_iter().rev() {
        server(ctx, &mut room, &name, up);
    }
    if let Some(tally) = super::faults::tally(app, path) {
        widgets::faults(ctx, &mut room, tally);
    }
    let dirty = app
        .workspace
        .buffer(path)
        .is_some_and(|open| open.new.dirty());
    if dirty {
        let size = ctx.styles.small(Role::Warn).size;
        let wide = |width: f32| Spec::default().width(width);
        let [rest, dot, _] = row_in(room, [Spec::fill(), wide(size), wide(ctx.tokens.sm)]);
        room = rest;
        let dot = square(dot, size);
        groove_ui_kit::widgets::icon(ctx, dot, Mark::Modified, Role::Warn);
    }
    let md = ctx.tokens.md;
    let style = ctx.styles.code(Role::Muted);
    Label::new(path, style)
        .cut_start()
        .draw(ctx, room.pad(Edges::across(md, md)));
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

/// A server's dot, in the colour of where it stands, then its name, from the right of `room`.
fn server(ctx: &mut Ctx, room: &mut Rect, name: &str, up: Up) {
    let role = match up {
        Up::Ready => Role::Ok,
        Up::Starting => Role::Working,
        Up::Down => Role::Bad,
    };
    let style = ctx.styles.code(Role::Faint);
    let (size, width) = (ctx.styles.small(role).size, ctx.measure(name, &style));
    let wide = |width: f32| Spec::default().width(width);
    let gaps = (wide(ctx.tokens.xs), wide(ctx.tokens.md));
    let [rest, dot, _, label, _] = row_in(
        *room,
        [Spec::fill(), wide(size), gaps.0, wide(width), gaps.1],
    );
    *room = rest;
    groove_ui_kit::widgets::icon(ctx, square(dot, size), Mark::Modified, role);
    Label::new(name, style).draw(ctx, label);
}
