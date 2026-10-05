//! What the opened sessions have done, under their rows: newest first, quiet.

use groove_controllers::{AppState, Told};
use groove_gfx::{Edges, Rect};
use groove_types::{Error, TimelineEvent, Timestamp};

mod line;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;

/// The header that folds it, then the lines themselves.
pub fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui) {
    let [head, body] = column_in(area, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    heading(ctx, head, app, ui);
    if !ui.rail.folded {
        lines(ctx, body, app, ui);
    }
}

/// The word that folds the feed, and the one that narrows it to the session in hand.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    groove_ui_kit::shape::rule_above(ctx, line, ctx.styles.line());
    let md = ctx.tokens.md;
    let mut room = line.pad(Edges::across(md, md));
    let scope = narrowed(ctx, &mut room, app, ui);
    groove_ui_kit::widgets::Heading::new("feed")
        .fold(!ui.rail.folded)
        .target(Target::Feed)
        .within(ctx, line, room);
    if let Some(scope) = scope {
        ctx.hit(scope, Target::FeedScope);
    }
}

/// Which sessions it shows, at the header's own end; returns the box a click narrows by.
fn narrowed(ctx: &mut Ctx, room: &mut Rect, app: &AppState, ui: &Ui) -> Option<Rect> {
    if app.session.open.len() < 2 {
        return None;
    }
    let (role, text) = match ui.rail.mine {
        true => (Role::Muted, "this session"),
        false => (Role::Ghost, "every session"),
    };
    Some(Label::new(text, ctx.styles.small(role)).right(ctx, room, 0.0))
}

/// The lines, scrolled and clipped to the room they have.
fn lines(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let shown = shown(app, ui);
    if shown.is_empty() {
        let style = ctx.styles.small(Role::Ghost);
        let room = body.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let [line, _] = column_in(
            room,
            [Spec::default().height(ctx.tokens.line), Spec::fill()],
        );
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
        |ctx, line, one| line::draw(ctx, line, one),
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
