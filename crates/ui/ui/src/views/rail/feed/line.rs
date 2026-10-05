//! One line of the feed: its age against a column, then what was done over what to.

use groove_gfx::{Align, Edges, Rect};
use groove_types::{SessionId, Timestamp};

use super::Line;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::{Label, ago};

/// What a line says: when it was, what was done, and what it was done to.
struct Said<'a> {
    at: Timestamp,
    act: &'a str,
    subject: &'a str,
    role: Role,
}

/// One line of the feed, drawn in the row the list gives it.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, one: &Line<'_>) {
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
    groove_ui_kit::widgets::busy(ctx, box_, Role::Working);
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
