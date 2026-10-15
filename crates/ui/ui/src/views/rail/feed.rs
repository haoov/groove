//! What the opened sessions have done, under their rows: newest first, quiet.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

mod line;
mod runs;

use runs::{Item, Run};

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
    let (tall, short) = (ctx.tokens.feed_row, ctx.tokens.row);
    let open = ui.rail.open;
    let at = (Scroller::Feed, ui.offset(Scroller::Feed));
    listed(
        ctx,
        body,
        at,
        &shown,
        |one| match one {
            Line::Under(_) => short,
            _ => tall,
        },
        |ctx, line, one| line::draw(ctx, line, one, (app, !ui.rail.mine, open)),
    );
}

/// One line of the feed: a job running now, a run of what happened, or one of an open run.
enum Line<'a> {
    Job(&'a str),
    Run(Run<'a>),
    Under(Item<'a>),
}

/// The running jobs first, then what happened, newest first, each run folded unless open.
fn shown<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let mut out: Vec<Line<'a>> = app
        .pending
        .iter()
        .rev()
        .map(|job| Line::Job(&job.label))
        .collect();
    let mut over = happened(app, ui);
    over.sort_by_key(|one| std::cmp::Reverse(one.at().seconds()));
    for run in runs::folded(over) {
        let opened = run.items.len() > 1 && ui.rail.open == Some(run.key());
        let under: Vec<Item<'a>> = match opened {
            true => run.items.clone(),
            false => Vec::new(),
        };
        out.push(Line::Run(run));
        out.extend(under.into_iter().map(Line::Under));
    }
    out
}

/// What is over: this session's events, or every session's, and Groove's own errors and notes.
fn happened<'a>(app: &'a AppState, ui: &Ui) -> Vec<Item<'a>> {
    let mut out: Vec<Item<'a>> = app
        .session
        .feed
        .iter()
        .filter(|one| !ui.rail.mine || app.session.selected.as_ref() == Some(&one.session))
        .map(Item::Event)
        .collect();
    out.extend(app.errors.iter().map(Item::Bad));
    out.extend(app.notes.iter().map(Item::Said));
    out
}
