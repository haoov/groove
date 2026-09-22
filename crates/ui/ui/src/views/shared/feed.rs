//! What the opened sessions have done, under their rows: newest first, quiet.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::TimelineEvent;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{ago, box_in, elide, hairline, row};

/// The header that folds it, then the lines themselves.
pub fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui) {
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    heading(ctx, head, app, ui);
    if ui.rail.folded {
        return;
    }
    let body = Rect::new(area.x, head.bottom(), area.w, area.bottom() - head.bottom());
    lines(ctx, body, app, ui);
}

/// The word that folds the feed, and the one that narrows it to the session in hand.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    hairline(
        ctx,
        Rect::new(line.x, line.y - ctx.tokens.hairline, line.w, 0.0),
        ctx.styles.line(),
    );
    grab(ctx, line);
    if ui.hover.as_ref() == Some(&Target::Feed) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, Target::Feed);
    let size = ctx.tokens.small;
    let box_ = box_in(line, line.x + ctx.tokens.md, size);
    let turn = match ui.rail.folded {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Faint));
    let style = ctx.styles.heading(Role::Faint);
    let at = ctx.tokens.md + size + ctx.tokens.xs;
    row(ctx, line, at, "FEED", style);
    narrowed(ctx, line, app, ui);
}

/// The band the pointer takes the feed's own edge by.
fn grab(ctx: &mut Ctx, line: Rect) {
    let thick = ctx.tokens.grab;
    let over = Rect::new(line.x, line.y - thick / 2.0, line.w, thick);
    ctx.hit(over, Target::Split(crate::layout::Edge::Feed));
}

/// Which sessions it shows, at the header's own end.
fn narrowed(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    if app.session.open.len() < 2 {
        return;
    }
    let role = match ui.rail.mine {
        true => Role::Muted,
        false => Role::Ghost,
    };
    let style = ctx.styles.small(role);
    let text = match ui.rail.mine {
        true => "this session",
        false => "every session",
    };
    let width = ctx.measure(text, &style);
    let at = line.right() - ctx.tokens.md - width;
    let box_ = Rect::new(at, line.y, width, line.h);
    row(ctx, box_, 0.0, text, style);
    ctx.hit(box_, Target::FeedScope);
}

/// The lines, scrolled and clipped to the room they have.
fn lines(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let shown = shown(app, ui);
    if shown.is_empty() {
        let style = ctx.styles.small(Role::Ghost);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.line);
        return row(ctx, line, ctx.tokens.md, "nothing yet", style);
    }
    let height = ctx.tokens.line;
    let extent = (height * shown.len() as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Feed, extent);
    let scroll = ui.rail.feed.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for one in shown {
            if y + height >= body.y && y <= body.bottom() {
                one_line(ctx, Rect::new(body.x, y, body.w, height), one, ctx.now);
            }
            y += height;
        }
    });
}

/// The lines the feed stands on: every session's, or the selected one's alone.
fn shown<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a TimelineEvent> {
    app.session
        .feed
        .iter()
        .filter(|one| !ui.rail.mine || app.session.selected.as_ref() == Some(&one.session))
        .collect()
}

/// One line: when it was, what it was, and what it was about.
fn one_line(ctx: &mut Ctx, line: Rect, one: &TimelineEvent, now: groove_types::Timestamp) {
    let quiet = ctx.styles.small(Role::Ghost);
    let when = ago(one.at.age_at(now));
    let width = ctx.measure(&when, &quiet);
    row(ctx, line, ctx.tokens.md, &when, quiet);
    let said = ctx.styles.small(Role::Muted);
    let at = ctx.tokens.md + width + ctx.tokens.sm;
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let text = elide(ctx, &told(one), &said, room);
    row(ctx, line, at, &text, said);
}

/// What one line says: its kind, and its subject where it has one.
fn told(one: &TimelineEvent) -> String {
    match one.subject.is_empty() {
        true => one.kind.label().to_string(),
        false => format!("{} {}", one.kind.label(), one.subject),
    }
}
