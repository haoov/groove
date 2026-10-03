//! Settings: the whole window, a section list with its search on the left, the form on the right.

pub mod draft;
mod form;
pub mod rows;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

pub use draft::Draft;
pub use rows::Section;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::widgets::{Button, Field, Row, Search};

/// What Settings remembers while it stands open.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SettingsUi {
    pub open: bool,
    pub section: Section,
    pub search: Field,
    /// The keyboard is in the search.
    pub typing: bool,
    /// How far the form is scrolled down.
    pub scroll: f32,
    /// A source being turned on, and one asked to be turned off, not yet confirmed.
    pub draft: Option<Draft>,
    pub leaving: Option<groove_types::ProviderId>,
    /// The shared repo asked to be let go, not yet confirmed.
    pub unsharing: bool,
    /// The id of the skill of the user's own asked deleted, not yet confirmed.
    pub deleting: Option<String>,
    /// The action whose new chord the next key is.
    pub binding: Option<crate::keymap::Action>,
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let mut window = ctx.window;
    groove_ui_kit::shape::ground(ctx, window, Ground::Work);
    let list = window.take_left(ctx.tokens.sidebar);
    groove_ui_kit::shape::ground(ctx, list, Ground::Band);
    let edge = list.right() - ctx.tokens.hairline;
    groove_ui_kit::shape::side_rule(ctx, list, edge, ctx.styles.line());
    sections(ctx, list, &ui.settings);
    form::draw(ctx, window, app, &ui.settings);
}

/// The search at the top of the list, then a row per section, the one up raised.
fn sections(ctx: &mut Ctx, mut list: Rect, settings: &SettingsUi) {
    let search = list.take_top(ctx.tokens.header);
    searched(ctx, search, settings);
    let up = Section::ALL.iter().position(|one| *one == settings.section);
    let up = up.filter(|_| settings.search.is_empty());
    let rows: Vec<Row<'_, Target>> = Section::ALL
        .iter()
        .enumerate()
        .map(|(at, section)| {
            let role = if up == Some(at) {
                Role::Text
            } else {
                Role::Muted
            };
            let style = ctx.styles.label(role);
            Row::new(ctx.tokens.md, section.label(), style)
                .target(Target::SettingsSection(*section))
        })
        .collect();
    groove_ui_kit::widgets::list(ctx, list, &rows, up);
}

fn searched(ctx: &mut Ctx, line: Rect, settings: &SettingsUi) {
    hairline(ctx, line, ctx.styles.line());
    let search = Search::new(&settings.search, Target::SettingsSearch, settings.typing);
    search.hint("search a setting").draw(ctx, line);
}

/// Where the sign-in's terminal stands: the lower half of the form.
pub(crate) fn login_pane(window: Rect, tokens: &Tokens) -> Rect {
    let mut form = window;
    form.take_left(tokens.sidebar);
    form.take_top(tokens.header);
    let mut body = form.pad(Edges::all(tokens.md));
    body.take_bottom(body.h / 2.0)
}

/// The bar over the form, with what takes the window back.
fn back(ctx: &mut Ctx, line: Rect) {
    hairline(ctx, line, ctx.styles.line());
    let mut room = line.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let word = Button::new(
        "back · esc",
        Target::SettingsBack,
        Role::Muted,
        ctx.styles.ground(),
    );
    word.left(ctx, &mut room, 0.0);
}
