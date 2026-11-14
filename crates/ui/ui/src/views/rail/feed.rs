//! What the opened sessions have done, as a log in its own window: newest first, quiet.

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
use groove_ui_kit::shape::{band, from_right};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{flat_row, marked_glass_tabs};

/// Its own window: the header that folds it, a bar tall and alone while folded, then the lines.
pub fn draw(ctx: &mut Ctx, window: Rect, app: &AppState, ui: &Ui) {
    let [head, body] = column_in(
        window,
        [Spec::default().height(ctx.tokens.bar), Spec::fill()],
    );
    heading(ctx, head, ui);
    if !ui.rail.folded {
        lines(ctx, body.pad(Edges::xy(0.0, ctx.tokens.xs)), app, ui);
    }
}

/// The word that folds the feed, and the tabs that narrow it to the session in hand: lit edge to
/// edge under the pointer, as the sessions' header is.
fn heading(ctx: &mut Ctx, line: Rect, ui: &Ui) {
    let row = flat_row(ctx, line, Target::Feed, false);
    let tall = ctx.tokens.row;
    let mut room = band(row, (row.h - tall) / 2.0, tall);
    room = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    narrowed(ctx, &mut room, ui);
    groove_ui_kit::widgets::Heading::new("feed")
        .fold(!ui.rail.folded)
        .within(ctx, row, room);
}

/// Which sessions it shows, as two tabs at the header's own end.
fn narrowed(ctx: &mut Ctx, room: &mut Rect, ui: &Ui) {
    let (sm, xs, md) = (ctx.tokens.sm, ctx.tokens.xs, ctx.tokens.md);
    let style = ctx.styles.small(Role::Text);
    let words = ctx.measure("this", &style) + ctx.measure("all", &style);
    // Each tab padded `md` on both sides, a gap between them, a margin at each end.
    let pads = (md + md) * 2.0 + xs + sm * 2.0;
    let place = from_right(room, words + pads, 0.0);
    let tabs = [
        (None, "this", Target::FeedScope(true)),
        (None, "all", Target::FeedScope(false)),
    ];
    marked_glass_tabs(ctx, place, &tabs, usize::from(!ui.rail.mine));
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
    let tall = ctx.tokens.feed_row;
    let open = ui.rail.open;
    let at = (Scroller::Feed, ui.offset(Scroller::Feed));
    listed(
        ctx,
        body,
        at,
        &shown,
        |_| tall,
        |ctx, line, one| line::draw(ctx, line, one, (app, !ui.rail.mine, open)),
    );
}

/// One line of the feed: a job running now, a run of what happened, or one of an open run, the last of it marked.
enum Line<'a> {
    Job(&'a str),
    Run(Run<'a>),
    Under(Item<'a>, bool),
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
        let last = under.len().saturating_sub(1);
        let lines = under.into_iter().enumerate();
        out.extend(lines.map(|(at, one)| Line::Under(one, at == last)));
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
