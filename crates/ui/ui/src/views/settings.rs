//! Settings: the whole window, a section list with its search on the left, the form on the right.

mod form;
pub mod rows;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

pub use rows::Section;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hairline, hoverable, square};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::{Field, Word};

/// What Settings remembers while it stands open.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SettingsUi {
    pub open: bool,
    pub section: Section,
    pub search: Field,
    /// The keyboard is in the search.
    pub typing: bool,
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let mut window = ctx.window;
    ctx.quad(window, ctx.styles.ground());
    let list = window.take_left(ctx.tokens.sidebar);
    ctx.quad(list, ctx.styles.band());
    let edge = Rect {
        x: list.right() - ctx.tokens.hairline,
        w: ctx.tokens.hairline,
        ..list
    };
    ctx.quad(edge, ctx.styles.line());
    sections(ctx, list, &ui.settings);
    form::draw(ctx, window, app, &ui.settings);
}

/// The search at the top of the list, then a row per section, the one up raised.
fn sections(ctx: &mut Ctx, mut list: Rect, settings: &SettingsUi) {
    let search = list.take_top(ctx.tokens.header);
    searched(ctx, search, settings);
    let searching = !settings.search.is_empty();
    for section in Section::ALL {
        let line = list.take_top(ctx.tokens.row);
        let up = !searching && settings.section == section;
        if up {
            ctx.quad(line, ctx.styles.raised());
        }
        hoverable(ctx, line, Target::SettingsSection(section));
        let role = match up {
            true => Role::Text,
            false => Role::Muted,
        };
        let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new(section.label(), ctx.styles.label(role)).draw(ctx, room);
    }
}

/// The search: its glass, and what is typed or what it offers.
fn searched(ctx: &mut Ctx, line: Rect, settings: &SettingsUi) {
    ctx.hit(line, Target::SettingsSearch);
    hairline(ctx, line, ctx.styles.line());
    let size = ctx.tokens.icon;
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let glass = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    let role = match settings.typing {
        true => Role::Text,
        false => Role::Ghost,
    };
    ctx.icon(glass, Mark::Search, 0, ctx.styles.color(role));
    let (text, style) = match (settings.typing, settings.search.is_empty()) {
        (true, _) => (settings.search.shown(), ctx.styles.body(Role::Text)),
        (false, true) => ("search a setting".to_string(), ctx.styles.body(Role::Ghost)),
        (false, false) => (
            settings.search.text().to_string(),
            ctx.styles.body(Role::Text),
        ),
    };
    row(ctx, room, 0.0, &text, style);
}

/// The bar over the form, with what takes the window back.
fn back(ctx: &mut Ctx, line: Rect) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let word = Word::new(
        "back · esc",
        Target::SettingsBack,
        Role::Muted,
        ctx.styles.ground(),
    );
    word.left(ctx, &mut room, 0.0);
}
