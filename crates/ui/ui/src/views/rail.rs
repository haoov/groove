mod feed;
mod item;

pub(crate) use item::asks_to;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::AttentionClass;

use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use crate::{Surface, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::{after_mark, square};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::{floating_pane, inset_row, lead};

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
    /// The feed's folded run that stands open.
    pub open: Option<RunKey>,
}

/// A folded run of the feed: what kind it folds, and when its newest one happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunKey {
    pub fold: Fold,
    pub newest: groove_types::Timestamp,
}

/// What a run of the feed folds together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    Error,
    Note,
    /// A kind of event; a turn's start and end are one.
    Event(groove_types::TimelineKind),
}

/// One row of the rail's list.
enum Entry<'a> {
    Session(&'a Open),
    /// A state's heading and how many sessions stand under it.
    Group(Group, usize),
    /// The routines heading and its count.
    Routines(usize),
}

/// The state a session stands under, the one that needs the user first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Group {
    NeedsYou,
    Working,
    Idle,
}

impl Group {
    fn label(self) -> &'static str {
        match self {
            Group::NeedsYou => "needs you",
            Group::Working => "working",
            Group::Idle => "idle",
        }
    }

    fn role(self) -> Role {
        match self {
            Group::NeedsYou => Role::Attention,
            Group::Working | Group::Idle => Role::Faint,
        }
    }
}

pub(crate) fn group_of(app: &AppState, open: &Open) -> Group {
    let class = app.agent.activity(&open.session.id).map(|one| one.class());
    match class {
        Some(AttentionClass::NeedsYou) => Group::NeedsYou,
        Some(AttentionClass::Moving) => Group::Working,
        Some(AttentionClass::ActWhenYouLook | AttentionClass::Quiet) | None => Group::Idle,
    }
}

/// A floating pane of glass: the Board row, the sessions by state, the feed, the footer.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let pane = floating_pane(ctx, ctx.app.layout.rail, Ground::Deep);
    let column = pane.pad(Edges::xy(0.0, ctx.tokens.sm));
    let tall = |height: f32| Spec::default().height(height);
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
    let up = ui.showing(app) == Surface::Board;
    let row = inset_row(ctx, rect, Target::Board, (up, None));
    let md = ctx.tokens.md;
    let mut room = row.pad(Edges::across(md, md));
    asking(ctx, &mut room, app.task.attention.len());
    lead(ctx, &mut room, Mark::Board, Role::Faint);
    Label::new("Board", ctx.styles.label(Role::Text)).draw(ctx, room);
}

/// How many items need the user, at the right of `room`.
fn asking(ctx: &mut Ctx, room: &mut Rect, count: usize) {
    if count > 0 {
        let style = ctx.styles.small(Role::Attention);
        Label::new(&count.to_string(), style).right(ctx, room, ctx.tokens.sm);
    }
}

/// The open sessions by state, then the routines' own, each in the order opened.
fn split(app: &AppState) -> (Vec<&Open>, Vec<&Open>) {
    let routine = |open: &&Open| open.session.kind.routine().is_none();
    let (mut sessions, routines): (Vec<&Open>, Vec<&Open>) =
        app.session.open.iter().partition(routine);
    sessions.sort_by_key(|open| group_of(app, open));
    (sessions, routines)
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
    let mut entries: Vec<Entry> = Vec::new();
    for (at, open) in sessions.iter().enumerate() {
        let group = group_of(app, open);
        if at == 0 || group_of(app, sessions[at - 1]) != group {
            let count = sessions.iter().filter(|one| group_of(app, one) == group);
            entries.push(Entry::Group(group, count.count()));
        }
        entries.push(Entry::Session(open));
    }
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
                Entry::Session(_) => item::height(ctx),
                Entry::Group(..) | Entry::Routines(_) => ctx.tokens.row,
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
            Entry::Group(group, count) => {
                let named = groove_ui_kit::widgets::Heading::new(group.label());
                let room = rect.pad(Edges::across(ctx.tokens.lg, ctx.tokens.lg));
                named
                    .count(*count)
                    .role(group.role())
                    .within(ctx, rect, room);
            }
            Entry::Routines(count) => heading(ctx, rect, *count, ui.rail.routines),
        },
    );
}

/// The heading that folds the routines' sessions, with how many there are.
fn heading(ctx: &mut Ctx, rect: Rect, count: usize, open: bool) {
    let row = inset_row(ctx, rect, Target::Routines, (false, None));
    let room = row.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    groove_ui_kit::widgets::Heading::new("routines")
        .count(count)
        .fold(open)
        .within(ctx, row, room);
}

fn footer(ctx: &mut Ctx, rect: Rect) {
    let md = ctx.tokens.md;
    let rule = rect.pad(Edges::across(md, md));
    groove_ui_kit::shape::top_rule(ctx, rule, ctx.styles.line());
    let line = inset_row(ctx, rect, Target::SettingsOpen, (false, None));
    let style = ctx.styles.small(Role::Faint);
    let mark = square(line.pad(Edges::across(md, 0.0)), ctx.tokens.icon);
    groove_ui_kit::widgets::icon(ctx, mark, Mark::Settings, Role::Faint);
    row(ctx, line, after_mark(ctx, md), "settings", style);
}
