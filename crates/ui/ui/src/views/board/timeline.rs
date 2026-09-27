//! The band under the columns: four weeks, and one bar a task.

mod bars;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{Day, Timestamp};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, square};
use groove_ui_kit::text::{Label, row};

/// The days the band shows, and how many stand before today.
pub(super) const DAYS: i64 = 28;
const BEFORE: i64 = 7;

/// How tall the band stands: its own bar alone, or as tall as the user left it.
pub(super) fn height(tokens: &groove_ui_kit::base::tokens::Tokens, app: &AppState, ui: &Ui) -> f32 {
    match shut(app, ui) {
        true => tokens.header,
        false => ui.split.band * tokens.scale,
    }
}

/// Whether the band is folded: by the user, or empty in a horizon they have not moved.
fn shut(app: &AppState, ui: &Ui) -> bool {
    ui.board.shut || (ui.board.horizon == 0 && bars::drawn(app, ui).is_empty())
}

/// The band: its own bar, then the grid and the bars over it.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui) {
    let mut body = band;
    let line = body.take_top(ctx.tokens.header);
    ctx.quad(band, ctx.styles.ground());
    rule(ctx, band);
    bar(ctx, line, app, ui);
    if shut(app, ui) {
        return;
    }
    grab(ctx, band);
    ctx.clipped(body, |ctx| {
        grid(ctx, body, ui);
        bars::draw(ctx, body, app, ui);
    });
    named_bar(ctx, app, ui);
}

/// The bar under the pointer, named where the pointer stands.
fn named_bar(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(Target::Bar(short_id)) = ctx.hover() else {
        return;
    };
    let Some(task) = app.task.get(short_id) else {
        return;
    };
    let title = Label::new(&task.title, ctx.styles.small(Role::Text));
    let (pad, at) = (ctx.tokens.sm, ui.at);
    let width = title.width(ctx) + pad * 2.0;
    let box_ = Rect::new(
        (at.0 + pad).min(ctx.window.right() - width),
        at.1 - ctx.tokens.row - pad,
        width,
        ctx.tokens.row,
    );
    ctx.layer();
    let (band, deep) = (ctx.styles.band(), ctx.styles.deep());
    Panel::default().ground(band).border(deep).draw(ctx, box_);
    title.draw(ctx, box_.pad(Edges::across(pad, pad)));
}

/// What a drag takes the band's own height by.
fn grab(ctx: &mut Ctx, band: Rect) {
    let grab = ctx.tokens.grab;
    let over = Rect::new(band.x, band.y - grab / 2.0, band.w, grab);
    ctx.hit(over, Target::Split(crate::layout::Edge::Band));
}

/// The rule that stands the band apart from the columns.
fn rule(ctx: &mut Ctx, band: Rect) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(band.x, band.y, band.w, thickness),
        ctx.styles.line(),
    );
}

/// What names the band and folds it away.
fn bar(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    let (sm, md, size) = (ctx.tokens.sm, ctx.tokens.md, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(md, 0.0));
    let caret = square(room.take_left(size), size);
    room.take_left(sm);
    let turn = match shut(app, ui) {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(caret, Mark::Down, turn, ctx.styles.color(Role::Ghost));
    ctx.hit(line, Target::Timeline);
    Label::new("TIMELINE", ctx.styles.heading(Role::Faint)).left(ctx, &mut room, md);
    let horizon = format!("4 weeks · {} – {}", named(first(ui)), named(last(ui)));
    Label::new(&horizon, ctx.styles.small(Role::Ghost)).draw(ctx, room);
}

/// The days, the weeks they fall in, and today among them.
fn grid(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let (thin, band) = (ctx.styles.line(), ctx.styles.band());
    let step = body.w / DAYS as f32;
    for day in 0..DAYS {
        let at = body.x + step * day as f32;
        let stands = first(ui).plus_days(day);
        if weekend(stands) {
            ctx.quad(Rect::new(at, body.y, step, body.h), band);
        }
    }
    for day in 0..=DAYS {
        let at = body.x + step * day as f32;
        let color = match day % 7 {
            0 => ctx.styles.border(),
            _ => thin,
        };
        ctx.quad(Rect::new(at, body.y, ctx.tokens.hairline, body.h), color);
        if day % 7 == 0 && day < DAYS {
            let style = ctx.styles.small(Role::Ghost);
            let text = named(first(ui).plus_days(day));
            let line = Rect::new(at, body.y, step * 2.0, ctx.tokens.line);
            row(ctx, line, ctx.tokens.xs, &text, style);
        }
    }
    let at = BEFORE - ui.board.horizon;
    if (0..DAYS).contains(&at) {
        let today = body.x + step * at as f32;
        let peach = ctx.styles.color(Role::Attention);
        ctx.quad(
            Rect::new(today, body.y, ctx.tokens.hairline * 2.0, body.h),
            peach,
        );
    }
}

/// The band's first day, and its last.
pub(super) fn first(ui: &Ui) -> Day {
    Timestamp::now().day().plus_days(ui.board.horizon - BEFORE)
}

fn last(ui: &Ui) -> Day {
    first(ui).plus_days(DAYS - 1)
}

fn weekend(day: Day) -> bool {
    matches!(day.days().rem_euclid(7), 2 | 3)
}

/// A day as the band names it.
fn named(day: Day) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS
        .get(usize::from(day.month).saturating_sub(1))
        .copied()
        .unwrap_or_default();
    format!("{} {month}", day.day)
}
