mod feed;
mod item;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use crate::{Surface, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{after_mark, hairline, hoverable, leading, ruled, square};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::{fold, icon};

/// What the rail remembers between frames.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RailUi {
    /// How far the rows are scrolled, in pixels.
    pub scroll: f32,
    /// How far the feed is scrolled, in pixels.
    pub feed: f32,
    /// The feed is folded to its own heading.
    pub folded: bool,
    /// The feed shows the selected session alone.
    pub mine: bool,
    /// The routines' sessions stand open under their heading.
    pub routines: bool,
}

/// One row of the rail's list.
enum Entry<'a> {
    Session(&'a Open),
    /// The routines heading and its count.
    Routines(usize),
}

/// The opened sessions, in the order opened. The Board row above, the footer below.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.app.layout.rail;
    ctx.quad(rect, ctx.styles.band());
    let mut column = rect;
    let edge = column.take_right(ctx.tokens.hairline);
    ctx.quad(edge, ctx.styles.line());

    let board = column.take_top(ctx.tokens.header);
    let foot = column.take_bottom(ctx.tokens.bar);
    board_row(ctx, app, ui, board);
    let band = ctx.app.layout.feed;
    let rows = Rect {
        h: (band.y - column.y).max(0.0),
        ..column
    };
    items(ctx, app, ui, rows);
    let feed = Rect {
        y: band.y,
        h: band.h,
        ..column
    };
    feed::draw(ctx, feed, app, ui);
    footer(ctx, foot);
}

/// The board. It carries the attention count when it is not zero.
fn board_row(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    if ctx.hovered(&Target::Board) {
        ctx.quad(rect, ctx.styles.hover());
    }
    ctx.hit(rect, Target::Board);
    let (md, size) = (ctx.tokens.md, ctx.tokens.icon);
    let mut room = rect.pad(Edges::across(md, md));
    asking(ctx, &mut room, app.task.attention.len());
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, mark, Mark::Board, Role::Faint);
    Label::new("Board", ctx.styles.label(Role::Text)).draw(ctx, room);
    hairline(ctx, rect, ctx.styles.line());
    if ui.showing(app) == Surface::Board {
        ruled(ctx, rect, ctx.styles.chosen());
    }
}

/// How many items need the user, at the right of `room`.
fn asking(ctx: &mut Ctx, room: &mut Rect, count: usize) {
    if count > 0 {
        let style = ctx.styles.small(Role::Attention);
        Label::new(&count.to_string(), style).right(ctx, room, ctx.tokens.sm);
    }
}

/// One item per open session, the routines' under their own heading, scrolled and clipped to `area`.
fn items(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let routine = |open: &&Open| open.session.kind.routine().is_some();
    let (routines, sessions): (Vec<_>, Vec<_>) = app.session.open.iter().partition(routine);
    let mut entries: Vec<Entry> = sessions.into_iter().map(Entry::Session).collect();
    if !routines.is_empty() {
        entries.push(Entry::Routines(routines.len()));
    }
    if ui.rail.routines {
        entries.extend(routines.into_iter().map(Entry::Session));
    }
    let items: Vec<_> = entries
        .into_iter()
        .map(|entry| {
            let tall = match &entry {
                Entry::Session(open) => item::height(ctx, app, &open.session.id),
                Entry::Routines(_) => ctx.tokens.row,
            };
            (entry, tall)
        })
        .collect();
    let at = (Scroller::Rail, ui.offset(Scroller::Rail));
    listed(
        ctx,
        area,
        at,
        &items,
        |(_, tall)| *tall,
        |ctx, rect, (entry, _)| match entry {
            Entry::Session(open) => item::draw(ctx, app, ui, rect, open),
            Entry::Routines(count) => heading(ctx, rect, *count, ui.rail.routines),
        },
    );
}

/// The heading that folds the routines' sessions, with how many there are.
fn heading(ctx: &mut Ctx, rect: Rect, count: usize, open: bool) {
    hoverable(ctx, rect, Target::Routines);
    let md = ctx.tokens.md;
    let mut room = rect.pad(Edges::across(md, md));
    fold(ctx, &mut room, open, Role::Faint);
    let said = format!("ROUTINES · {count}");
    Label::new(&said, ctx.styles.heading(Role::Faint)).draw(ctx, room);
}

fn footer(ctx: &mut Ctx, rect: Rect) {
    let rule = ctx.styles.line();
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - rect.h, rect.w, rect.h),
        rule,
    );
    hoverable(ctx, rect, Target::SettingsOpen);
    let style = ctx.styles.small(Role::Faint);
    let md = ctx.tokens.md;
    let mark = leading(ctx, rect, rect.x + md);
    ctx.icon(mark, Mark::Settings, 0, style.color);
    row(ctx, rect, after_mark(ctx, md), "settings", style);
}
