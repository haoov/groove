//! The band above the rows: on the diff, the whole change and its views; on a file, its path.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::DiffView;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Tab, files};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::{hairline, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{self, Text, changes};

/// The band over the diff or over the open file.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    hairline(ctx, band, ctx.styles.line());
    let mut room = band;
    if ui.session.tab == Tab::Diff {
        switch(ctx, &mut room, ui.session.view);
        return totals(ctx, room, app);
    }
    if let Some(counts) = super::row::counted(app, path) {
        changes(ctx, &mut room, counts, |ctx, role| ctx.styles.code(role));
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

/// The three views from the right of `room`, the current one raised.
fn switch(ctx: &mut Ctx, room: &mut Rect, current: DiffView) {
    for view in DiffView::ALL.into_iter().rev() {
        let tab = widgets::Tab::new(view.label(), Target::View(view), view == current);
        let tab = tab.text(Text::Body).quiet(Role::Faint).tight();
        tab.right(ctx, room, 0.0);
    }
}

/// How many files the change touches, and what it added and removed in all.
fn totals(ctx: &mut Ctx, mut room: Rect, app: &AppState) {
    let files = files::changed(app);
    let added = files.iter().map(|file| file.added).sum();
    let deleted = files.iter().map(|file| file.deleted).sum();
    changes(ctx, &mut room, (added, deleted), |ctx, role| {
        ctx.styles.code(role)
    });
    let said = match files.len() {
        1 => "1 file".to_string(),
        many => format!("{many} files"),
    };
    let md = ctx.tokens.md;
    Label::new(&said, ctx.styles.body(Role::Muted)).draw(ctx, room.pad(Edges::across(md, md)));
}
