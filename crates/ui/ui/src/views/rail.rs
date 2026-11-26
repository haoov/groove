mod drag;
mod feed;
mod item;
mod kind;
mod order;

pub use drag::Slot;
pub(crate) use drag::{landing, slot_at, top_of};
pub(crate) use item::{asks_to, state_of};
pub use order::Shut;
pub(crate) use order::shown;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::SessionId;

use self::order::Entry;
use groove_gfx::{Edges, Rect};
use groove_types::Section;

use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use crate::{Surface, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::top_rule;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, flat_row, ground_under, lead, panel_head, solid_pane};

/// What the rail remembers between frames.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RailUi {
    /// How far the rows are scrolled, in pixels.
    pub scroll: f32,
    /// How far the feed is scrolled, in pixels.
    pub feed: f32,
    /// The feed is folded to its own heading.
    pub folded: bool,
    /// The sessions' window is folded to its own header.
    pub stowed: bool,
    /// The feed shows the selected session alone.
    pub mine: bool,
    /// Where a carried session would land.
    pub drop: Option<Slot>,
    pub shut: Shut,
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

/// Two opaque windows in the rail's place: the sessions' header, the Board row and the
/// sessions by state, then the feed under them.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let layout = ctx.app.layout;
    ground_under(ctx, layout.rail, Ground::Deep);
    groove_ui_kit::widgets::ground_lattice(ctx, layout.rail);
    if !over(app, ui) {
        windows(ctx, app, ui);
    }
}

/// Both windows folded under the modes layout: the session's middle runs across the column, and
/// their heads stand over it, drawn after it as the sidebar's is.
pub fn over(app: &AppState, ui: &Ui) -> bool {
    let session = ui.showing(app) == Surface::Session && app.session.selected().is_some();
    ui.modes && session && ui.rail.stowed && ui.rail.folded
}

/// The two windows on the rail's ground.
pub fn windows(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let layout = ctx.app.layout;
    solid_pane(ctx, layout.sessions);
    solid_pane(ctx, layout.feed);
    let [head, board, rows] = parts_of(&ctx.tokens, layout.sessions);
    panel_head(ctx, head, ui.rail.stowed);
    header(ctx, app, ui, head);
    feed::draw(ctx, layout.feed, app, ui);
    if ui.rail.stowed {
        return;
    }
    board_row(ctx, app, ui, board);
    top_rule(ctx, rows, ctx.styles.line());
    let rows = rows_in(&ctx.tokens, rows);
    items(ctx, app, ui, rows);
    ctx.clipped(rows, |ctx| drag::draw(ctx, app, ui, rows));
}

/// The sessions' window cut in its header, its Board row and its list.
fn parts_of(tokens: &Tokens, sessions: Rect) -> [Rect; 3] {
    let tall = |height: f32| Spec::default().height(height);
    column_in(
        sessions,
        [tall(tokens.bar), tall(tokens.header), Spec::fill()],
    )
}

fn rows_in(tokens: &Tokens, rows: Rect) -> Rect {
    rows.pad(Edges::xy(0.0, tokens.xs))
}

/// The room the rail's list stands in, within the sessions' window.
pub(crate) fn rows_of(tokens: &Tokens, sessions: Rect) -> Rect {
    let [_, _, rows] = parts_of(tokens, sessions);
    rows_in(tokens, rows)
}

/// The window's name and how many sessions are open, a click on it folding the window to it,
/// and the `+` that opens a new explorer.
fn header(ctx: &mut Ctx, app: &AppState, ui: &Ui, line: Rect) {
    let md = ctx.tokens.md;
    let row = flat_row(ctx, line, Target::SessionsFold, false);
    let mut room = row.pad(Edges::across(md, ctx.tokens.sm));
    let plus = Button::icon(Mark::Add, 0, Target::NewExplorer, Role::Faint);
    plus.right(ctx, &mut room, 0.0);
    let asking = ask_count(app);
    let asks = format!("{asking} ask");
    groove_ui_kit::widgets::Heading::new("sessions")
        .count(app.session.open.len())
        .also((asking > 0).then_some((asks.as_str(), Role::Attention)))
        .fold(!ui.rail.stowed)
        .within(ctx, row, room);
}

/// How many of the rail's sessions ask the user.
fn ask_count(app: &AppState) -> usize {
    let asks = |open: &&Open| {
        let activity = app.agent.activity(&open.session.id);
        activity.is_some_and(|one| one.class() == groove_types::AttentionClass::NeedsYou)
    };
    app.session.open.iter().filter(asks).count()
}

/// The board. It carries the attention count when it is not zero.
fn board_row(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect) {
    let up = ui.showing(app) == Surface::Board;
    let row = flat_row(ctx, rect, Target::Board, up);
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

/// One item per line of the rail, scrolled and clipped to `area`; the carried one ghosted.
fn items(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let tokens = ctx.tokens;
    let dragging = ui.rail.drop.is_some();
    let items: Vec<_> = order::entries(app, dragging, ui.rail.shut)
        .into_iter()
        .map(|entry| {
            let tall = drag::height_of(&tokens, &entry);
            (entry, tall)
        })
        .collect();
    let at = (Scroller::Rail, ui.offset(Scroller::Rail));
    let carried = drag::carried(ui).filter(|_| dragging).map(|one| one.0);
    listed(
        ctx,
        area,
        at,
        &items,
        |(_, tall)| *tall,
        |ctx, rect, (entry, _)| {
            let ghost = ghosted(entry, carried);
            if ghost {
                ctx.fading(groove_ui_kit::base::tokens::GHOST);
            }
            line(ctx, app, ui, rect, entry);
            if ghost {
                ctx.fading(1.0);
            }
        },
    );
}

/// Whether `entry` is the carried session, or one of its children.
fn ghosted(entry: &Entry, carried: Option<&SessionId>) -> bool {
    let Some(carried) = carried else {
        return false;
    };
    match entry {
        Entry::Heading(..) => false,
        Entry::Session(open) => &open.session.id == carried,
        Entry::Child { open, .. } => open.state.parent.as_ref() == Some(carried),
    }
}

fn line(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, entry: &Entry) {
    match entry {
        Entry::Session(open) => item::draw(ctx, app, ui, rect, open),
        Entry::Child { open, last } => item::draw_child(ctx, app, ui, rect, open, *last),
        Entry::Heading(section, count) => {
            let named = groove_ui_kit::widgets::Heading::new(label_of(*section))
                .count(*count)
                .fold(!ui.rail.shut.holds(*section))
                .target(Target::SectionFold(*section));
            let room = rect.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
            named.within(ctx, rect, room);
        }
    }
}

fn label_of(section: Section) -> &'static str {
    match section {
        Section::Active => "active",
        Section::Paused => "paused",
    }
}
