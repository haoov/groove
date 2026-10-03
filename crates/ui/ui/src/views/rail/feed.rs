//! What the opened sessions have done, under their rows: newest first, quiet.

use groove_controllers::{AppState, Told};
use groove_gfx::{Align, Edges, Rect};
use groove_types::{Error, SessionId, TimelineEvent, Timestamp};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::motion::turn;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hairline, hoverable};
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::fold;

/// The header that folds it, then the lines themselves.
pub fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui) {
    let mut body = area;
    let head = body.take_top(ctx.tokens.row);
    heading(ctx, head, app, ui);
    if !ui.rail.folded {
        lines(ctx, body, app, ui);
    }
}

/// The word that folds the feed, and the one that narrows it to the session in hand.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    hairline(
        ctx,
        Rect::new(line.x, line.y - ctx.tokens.hairline, line.w, 0.0),
        ctx.styles.line(),
    );
    hoverable(ctx, line, Target::Feed);
    grab(ctx, line);
    let md = ctx.tokens.md;
    let mut room = line.pad(Edges::across(md, md));
    narrowed(ctx, &mut room, app, ui);
    fold(ctx, &mut room, !ui.rail.folded, Role::Faint);
    Label::new("FEED", ctx.styles.heading(Role::Faint)).draw(ctx, room);
}

/// The band the pointer takes the feed's own edge by.
fn grab(ctx: &mut Ctx, line: Rect) {
    let thick = ctx.tokens.grab;
    let over = Rect::new(line.x, line.y - thick / 2.0, line.w, thick);
    ctx.hit(over, Target::Split(crate::layout::Edge::Feed));
}

/// Which sessions it shows, at the header's own end.
fn narrowed(ctx: &mut Ctx, room: &mut Rect, app: &AppState, ui: &Ui) {
    if app.session.open.len() < 2 {
        return;
    }
    let (role, text) = match ui.rail.mine {
        true => (Role::Muted, "this session"),
        false => (Role::Ghost, "every session"),
    };
    let box_ = Label::new(text, ctx.styles.small(role)).right(ctx, room, 0.0);
    ctx.hit(box_, Target::FeedScope);
}

/// The lines, scrolled and clipped to the room they have.
fn lines(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let shown = shown(app, ui);
    if shown.is_empty() {
        let style = ctx.styles.small(Role::Ghost);
        let line = body
            .pad(Edges::across(ctx.tokens.md, ctx.tokens.md))
            .take_top(ctx.tokens.line);
        Label::new("nothing yet", style).draw(ctx, line);
        return;
    }
    let height = ctx.tokens.feed_row;
    let at = (Scroller::Feed, ui.offset(Scroller::Feed));
    listed(
        ctx,
        body,
        at,
        &shown,
        |_| height,
        |ctx, line, one| one_line(ctx, line, one),
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

fn one_line(ctx: &mut Ctx, line: Rect, one: &Line<'_>) {
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
            reachable(ctx, line, &event.session);
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
fn reachable(ctx: &mut Ctx, line: Rect, session: &SessionId) {
    let target = Target::FeedLine(session.clone());
    hoverable(ctx, line, target);
}

/// A job the user waits on, with the mark that keeps turning.
fn running(ctx: &mut Ctx, line: Rect, label: &str) {
    let (mut top, _) = halves(ctx, line);
    let size = ctx.tokens.small;
    let age = top.take_left(column(ctx));
    top.take_left(ctx.tokens.sm);
    let box_ = age.align((size, size), Align::End, Align::Center);
    let color = ctx.styles.color(Role::Working);
    ctx.icon(box_, Mark::Busy, turn(ctx.tick), color);
    Label::new(label, ctx.styles.strong(Role::Working)).draw(ctx, top);
}

/// The age against its own column, then the act over the subject.
fn over(ctx: &mut Ctx, line: Rect, said: Said<'_>) {
    let (mut top, mut under) = halves(ctx, line);
    let mut age = top.take_left(column(ctx));
    let gap = column(ctx) + ctx.tokens.sm;
    top.take_left(ctx.tokens.sm);
    under.take_left(gap);
    let when = ago(said.at.age_at(ctx.now));
    Label::new(&when, ctx.styles.small(Role::Ghost)).right(ctx, &mut age, 0.0);
    Label::new(said.act, ctx.styles.strong(said.role)).draw(ctx, top);
    if !said.subject.is_empty() {
        Label::new(said.subject, ctx.styles.small(Role::Muted)).draw(ctx, under);
    }
}

/// The line's two halves, each short of the right margin.
fn halves(ctx: &Ctx, line: Rect) -> (Rect, Rect) {
    let mut top = line.pad(Edges::across(0.0, ctx.tokens.md));
    let under = top.take_bottom(line.h / 2.0);
    (top, under)
}

/// Where the age's column ends, which every text stands after.
fn column(ctx: &Ctx) -> f32 {
    ctx.tokens.md + ctx.tokens.feed_age
}
