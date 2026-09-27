//! The band above the rows: the path, what it changed, and the three views.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::DiffView;

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, square};
use crate::text::Label;
use crate::views::session::files;
use crate::widgets::changes;

/// The file's path, what it changed, and which view it is drawn in.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    let mut room = band;
    switch(ctx, &mut room, ui.session.view);
    if let Some(counts) = counted(app, path) {
        changes(ctx, &mut room, counts, |ctx, role| ctx.styles.code(role));
    }
    let dirty = app
        .workspace
        .opened
        .as_ref()
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
    hairline(ctx, band, ctx.styles.line());
}

/// The three views from the right of `room`, the current one raised.
fn switch(ctx: &mut Ctx, room: &mut Rect, current: DiffView) {
    let (pad, gap) = (ctx.tokens.sm, ctx.tokens.xs);
    for view in DiffView::ALL.into_iter().rev() {
        let role = match view == current {
            true => Role::Text,
            false => Role::Faint,
        };
        let label = Label::new(view.label(), ctx.styles.small(role));
        let box_ = room.take_right(label.width(ctx) + pad * 2.0);
        room.take_right(gap);
        if view == current {
            ctx.quad(box_, ctx.styles.raised());
        }
        label.draw(ctx, box_.pad(Edges::across(pad, pad)));
        ctx.hit(box_, Target::View(view));
    }
}

/// The summary's counts for this path.
fn counted(app: &AppState, path: &str) -> Option<(u32, u32)> {
    files::changed(app)
        .iter()
        .find(|file| file.path == path)
        .map(|file| (file.added, file.deleted))
}
