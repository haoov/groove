//! The band under the columns: four weeks, and one bar a task.

mod bars;

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{Day, Timestamp};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, leading, row};

/// The days the band shows, and how many stand before today.
pub(super) const DAYS: i64 = 28;
const BEFORE: i64 = 7;

/// How tall the band stands: its own bar alone, or as tall as the user left it.
pub(super) fn tall(tokens: &crate::tokens::Tokens, app: &AppState, ui: &Ui) -> f32 {
    match shut(app, ui) {
        true => tokens.header,
        false => ui.split.band * tokens.scale,
    }
}

/// Whether the band is folded away: by the user, or for want of anything in a horizon
/// they have not moved.
fn shut(app: &AppState, ui: &Ui) -> bool {
    ui.board.shut || (ui.board.horizon == 0 && bars::drawn(app, ui).is_empty())
}

/// The band: its own bar, then the grid and the bars over it.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui) {
    let line = Rect::new(band.x, band.y, band.w, ctx.tokens.header);
    ctx.quad(band, ctx.styles.ground());
    rule(ctx, band);
    bar(ctx, line, app, ui);
    if shut(app, ui) {
        return;
    }
    grab(ctx, band);
    let body = Rect::new(band.x, line.bottom(), band.w, band.bottom() - line.bottom());
    ctx.clipped(body, |ctx| {
        grid(ctx, body, ui);
        bars::draw(ctx, body, app, ui);
    });
    named_bar(ctx, app, ui);
}

/// The bar under the pointer, named where the pointer stands.
fn named_bar(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(Target::Bar(short_id)) = &ui.hover else {
        return;
    };
    let Some(task) = app.task.get(short_id) else {
        return;
    };
    let style = ctx.styles.small(Role::Text);
    let (pad, at) = (ctx.tokens.sm, ui.at);
    let width = ctx.measure(&task.title, &style) + pad * 2.0;
    let box_ = Rect::new(
        (at.0 + pad).min(ctx.layout.window.right() - width),
        at.1 - ctx.tokens.row - pad,
        width,
        ctx.tokens.row,
    );
    ctx.layer();
    ctx.quad(box_, ctx.styles.band());
    ctx.border(box_, ctx.styles.deep());
    row(ctx, box_, pad, &task.title, style);
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
    let box_ = leading(ctx, line, line.x + ctx.tokens.md);
    let turn = match shut(app, ui) {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Ghost));
    ctx.hit(line, Target::Timeline);
    let at = line.x + after_mark(ctx, ctx.tokens.md);
    let style = ctx.styles.heading(Role::Faint);
    let width = ctx.measure("TIMELINE", &style);
    row(
        ctx,
        Rect::new(at, line.y, width, line.h),
        0.0,
        "TIMELINE",
        style,
    );
    let horizon = format!("4 weeks · {} – {}", named(first(ui)), named(last(ui)));
    let quiet = ctx.styles.small(Role::Ghost);
    let at = at + width + ctx.tokens.md;
    row(
        ctx,
        Rect::new(at, line.y, line.w, line.h),
        0.0,
        &horizon,
        quiet,
    );
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
