//! The band above the rows: the path, what it changed, and the three views.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::DiffView;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Tab, files};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hairline, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{self, Text, changes};

/// The file's path, what it changed, and which view it is drawn in.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    hairline(ctx, band, ctx.styles.line());
    let mut room = band;
    if ui.session.tab == Tab::Diff {
        switch(ctx, &mut room, ui.session.view);
    }
    if let Some(counts) = counted(app, path) {
        changes(ctx, &mut room, counts, |ctx, role| ctx.styles.code(role));
    }
    let dirty = app
        .workspace
        .buffer(path)
        .is_some_and(|open| open.new.dirty());
    if dirty {
        let size = ctx.styles.small(Role::Warn).size;
        room.take_right(ctx.tokens.sm);
        let dot = square(room.take_right(size), size);
        ctx.icon(dot, Mark::Modified, 0, ctx.styles.color(Role::Warn));
    }
    let md = ctx.tokens.md;
    let style = ctx.styles.code(Role::Muted);
    Label::new(path, style)
        .cut_start()
        .draw(ctx, room.pad(Edges::across(md, md)));
}

/// The three views from the right of `room`, the current one raised.
fn switch(ctx: &mut Ctx, room: &mut Rect, current: DiffView) {
    for view in DiffView::ALL.into_iter().rev() {
        let tab = widgets::Tab::new(view.label(), Target::View(view), view == current);
        let tab = tab.text(Text::Small).quiet(Role::Faint).tight();
        tab.right(ctx, room, 0.0);
    }
}

/// The summary's counts for this path.
fn counted(app: &AppState, path: &str) -> Option<(u32, u32)> {
    files::changed(app)
        .iter()
        .find(|file| file.path == path)
        .map(|file| (file.added, file.deleted))
}
