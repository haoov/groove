mod feed;
mod item;

pub(crate) use item::asks_to;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use crate::{Surface, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::{after_mark, hairline, hoverable, leading, ruled};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::lead;
use groove_ui_kit::widgets::{Side, pane};

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
    pane(ctx, rect, Ground::Band, Side::Right);
    let tall = |height: f32| Spec::default().height(height);
    let [column, _] = row_in(
        rect,
        [Spec::fill(), Spec::default().width(ctx.tokens.hairline)],
    );
    let parts = [
        tall(ctx.tokens.header),
        Spec::fill(),
        tall(ctx.app.layout.feed.h),
        tall(ctx.tokens.bar),
    ];
    let [board, rows, feed, foot] = column_in(column, parts);
    board_row(ctx, app, ui, board);
    items(ctx, app, ui, rows);
    feed::draw(ctx, feed, app, ui);
    footer(ctx, foot);
}

/// The board. It carries the attention count when it is not zero.
fn board_row(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    hoverable(ctx, rect, Target::Board);
    let (md, _size) = (ctx.tokens.md, ctx.tokens.icon);
    let mut room = rect.pad(Edges::across(md, md));
    asking(ctx, &mut room, app.task.attention.len());
    lead(ctx, &mut room, Mark::Board, Role::Faint);
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

/// The open sessions, then the routines' own, each in the order opened.
fn split(app: &AppState) -> (Vec<&Open>, Vec<&Open>) {
    let routine = |open: &&Open| open.session.kind.routine().is_none();
    app.session.open.iter().partition(routine)
}

/// The sessions the rail shows, in its order: the routines' only while their heading is open.
pub(crate) fn shown<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a Open> {
    let (mut sessions, routines) = split(app);
    if ui.rail.routines {
        sessions.extend(routines);
    }
    sessions
}

/// One item per open session, the routines' under their own heading, scrolled and clipped to `area`.
fn items(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let (sessions, routines) = split(app);
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
    groove_ui_kit::widgets::Heading::new("routines")
        .count(count)
        .fold(open)
        .target(Target::Routines)
        .draw(ctx, rect);
}

fn footer(ctx: &mut Ctx, rect: Rect) {
    groove_ui_kit::shape::top_rule(ctx, rect, ctx.styles.line());
    hoverable(ctx, rect, Target::SettingsOpen);
    let style = ctx.styles.small(Role::Faint);
    let md = ctx.tokens.md;
    let mark = leading(ctx, rect, rect.x + md);
    groove_ui_kit::widgets::icon(ctx, mark, Mark::Settings, Role::Faint);
    row(ctx, rect, after_mark(ctx, md), "settings", style);
}
