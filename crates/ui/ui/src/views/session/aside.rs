//! The modes layout's sidebar, one for every tab: its pane, a head that folds it, the tab's body,
//! and one scrollbar over whatever the body scrolls.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::SessionKind;
use groove_ui_kit::shape::from_top;
use groove_ui_kit::widgets::{Heading, flat_row, flowed, panel_head, scrollbar, solid_pane};

use super::{Mode, agent_pane, files, overview};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// The sidebar in `rect`, folded to its head unless `shown`.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, open: &Open, (rect, shown): (Rect, bool)) {
    solid_pane(ctx, rect);
    let mut room = rect;
    let top = from_top(&mut room, ctx.tokens.bar, 0.0);
    panel_head(ctx, top, !shown);
    let row = flat_row(ctx, top, Target::AsideFold, false);
    match ui.session.mode {
        Mode::Diff => files::moded::head(ctx, app, row, shown),
        Mode::Agent => Heading::new("agent").fold(shown).draw(ctx, row),
        _ => Heading::new(kind(open)).fold(shown).draw(ctx, row),
    }
    if !shown {
        return;
    }
    let (which, body, extent) = match ui.session.mode {
        Mode::Diff => {
            let (body, extent) = files::moded::body(ctx, app, ui, room);
            (Scroller::Files, body, extent)
        }
        Mode::Agent => flow(ctx, ui, (room, room), |ctx, at| {
            agent_pane::aside::body(ctx, app, at);
        }),
        _ => {
            let side = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
            flow(ctx, ui, (room, side), |ctx, at| {
                overview::moded::aside::body(ctx, app, open, at);
            })
        }
    };
    if let Some(track) = scrollbar(ctx, body, (ui.offset(which), extent)) {
        ctx.hit(track, Target::Thumb(which));
    }
}

/// A body that is not a list, laid in `inside` at the sidebar's own offset.
fn flow<'c>(
    ctx: &mut Ctx<'c>,
    ui: &Ui,
    (room, inside): (Rect, Rect),
    draw: impl FnOnce(&mut Ctx<'c>, &mut Rect),
) -> (Scroller, Rect, f32) {
    let extent = flowed(ctx, inside, ui.offset(Scroller::Aside), draw);
    ctx.app.hits.scrolls(Scroller::Aside, extent);
    (Scroller::Aside, room, extent)
}

/// What the session is, as the head names it.
fn kind(open: &Open) -> &'static str {
    match open.session.kind {
        SessionKind::Review { .. } => "review",
        SessionKind::Explorer => "explorer",
        _ => "task",
    }
}
