//! What the opened sessions have done, under their rows: newest first, quiet.

use groove_controllers::{AppState, Told};
use groove_gfx::Rect;
use groove_types::{Error, SessionId, TimelineEvent, Timestamp};

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::{Scroller, Target};
use crate::base::mark::Mark;
use crate::base::motion::turn;
use crate::base::style::Role;
use crate::shape::{box_in, hairline};
use crate::text::{ago, elide, row};
use crate::widgets::scrolled;

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
    let height = ctx.tokens.feed_row;
    let at = (Scroller::Feed, ui.offset(Scroller::Feed));
    scrolled(
        ctx,
        body,
        at,
        &shown,
        |_| height,
        |ctx, line, one| one_line(ctx, line, one, ui),
    );
}

/// One line of the feed: a job running now, or something that happened.
enum Line<'a> {
    Job(&'a str),
    Bad(&'a Told<Error>),
    Said(&'a Told<String>),
    Event(&'a TimelineEvent),
}

/// The running jobs first, then what happened, newest first.
fn shown<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let mut out: Vec<Line<'a>> = app
        .pending
        .iter()
        .rev()
        .map(|job| Line::Job(&job.label))
        .collect();
    let mut over = happened(app, ui);
    over.sort_by_key(|(at, _)| std::cmp::Reverse(at.seconds()));
    out.extend(over.into_iter().map(|(_, line)| line));
    out
}

/// What is over, each at its own moment: this session's lines, or every session's.
fn happened<'a>(app: &'a AppState, ui: &Ui) -> Vec<(Timestamp, Line<'a>)> {
    let mut out: Vec<(Timestamp, Line<'a>)> = app
        .session
        .feed
        .iter()
        .filter(|one| !ui.rail.mine || app.session.selected.as_ref() == Some(&one.session))
        .map(|one| (one.at, Line::Event(one)))
        .collect();
    out.extend(app.errors.iter().map(|one| (one.at, Line::Bad(one))));
    out.extend(app.notes.iter().map(|one| (one.at, Line::Said(one))));
    out
}

/// What a line says: when it was, what was done, and what it was done to.
struct Said<'a> {
    at: Timestamp,
    act: &'a str,
    subject: &'a str,
    role: Role,
}

fn one_line(ctx: &mut Ctx, line: Rect, one: &Line<'_>, ui: &Ui) {
    match one {
        Line::Job(label) => running(ctx, line, label),
        Line::Bad(bad) => over(
            ctx,
            line,
            Said {
                at: bad.at,
                act: "error",
                subject: &bad.message,
                role: Role::Bad,
            },
        ),
        Line::Said(said) => over(
            ctx,
            line,
            Said {
                at: said.at,
                act: "note",
                subject: &said.what,
                role: Role::Accent,
            },
        ),
        Line::Event(event) => {
            reachable(ctx, line, &event.session, ui);
            over(
                ctx,
                line,
                Said {
                    at: event.at,
                    act: event.kind.label(),
                    subject: &event.subject,
                    role: Role::Accent,
                },
            );
        }
    }
}

/// The line takes the pointer to the session it belongs to.
fn reachable(ctx: &mut Ctx, line: Rect, session: &SessionId, ui: &Ui) {
    let target = Target::FeedLine(session.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
}

/// A job the user waits on, with the mark that keeps turning.
fn running(ctx: &mut Ctx, line: Rect, label: &str) {
    let style = ctx.styles.strong(Role::Working);
    let top = Rect::new(line.x, line.y, line.w, line.h / 2.0);
    let size = ctx.tokens.small;
    let box_ = box_in(top, line.x + column(ctx) - size, size);
    ctx.icon(
        box_,
        Mark::Busy,
        turn(ctx.tick),
        ctx.styles.color(Role::Working),
    );
    let at = column(ctx) + ctx.tokens.sm;
    let text = elide(ctx, label, &style, room(ctx, line, at));
    row(ctx, top, at, &text, style);
}

/// The age against its own column, then the act over the subject.
fn over(ctx: &mut Ctx, line: Rect, said: Said<'_>) {
    let half = line.h / 2.0;
    let top = Rect::new(line.x, line.y, line.w, half);
    let quiet = ctx.styles.small(Role::Ghost);
    let when = ago(said.at.age_at(ctx.now));
    let width = ctx.measure(&when, &quiet);
    row(ctx, top, column(ctx) - width, &when, quiet);
    let at = column(ctx) + ctx.tokens.sm;
    let act = ctx.styles.strong(said.role);
    let text = elide(ctx, said.act, &act, room(ctx, line, at));
    row(ctx, top, at, &text, act);
    if said.subject.is_empty() {
        return;
    }
    let under = Rect::new(line.x, line.y + half, line.w, half);
    let style = ctx.styles.small(Role::Muted);
    let text = elide(ctx, said.subject, &style, room(ctx, line, at));
    row(ctx, under, at, &text, style);
}

/// Where the age's column ends, which every text stands after.
fn column(ctx: &Ctx) -> f32 {
    ctx.tokens.md + ctx.tokens.feed_age
}

fn room(ctx: &Ctx, line: Rect, at: f32) -> f32 {
    (line.w - at - ctx.tokens.md).max(0.0)
}
