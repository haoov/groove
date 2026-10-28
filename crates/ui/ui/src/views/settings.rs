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
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Field, Row, Search};
use groove_ui_kit::widgets::{floating_pane, glass_field, glass_list, ground_under};

/// What Settings remembers while it stands open.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SettingsUi {
    pub open: bool,
    /// Setup was offered at start, or the user acted before it could be.
    pub offered: bool,
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
    ground_under(ctx, ctx.window, Ground::Deep);
    let [list, window] = row_in(
        ctx.window,
        [Spec::default().width(ctx.tokens.sidebar), Spec::fill()],
    );
    let pane = floating_pane(ctx, list, Ground::Deep);
    sections(ctx, pane, app, &ui.settings);
    form::draw(ctx, window, app, &ui.settings);
}

/// The title and what takes the window back, the search, then a row per section, the one up held.
fn sections(ctx: &mut Ctx, pane: Rect, app: &AppState, settings: &SettingsUi) {
    let tall = |height: f32| Spec::default().height(height);
    let column = pane.pad(Edges::xy(0.0, ctx.tokens.sm));
    let [head, search, list] = column_in(
        column,
        [
            tall(ctx.tokens.header),
            tall(ctx.tokens.row + ctx.tokens.sm),
            Spec::fill(),
        ],
    );
    back(ctx, head);
    searched(ctx, search, settings);
    let up = Section::ALL.iter().position(|one| *one == settings.section);
    let up = up.filter(|_| settings.search.is_empty());
    let counts: Vec<String> = Section::ALL.iter().map(|one| needs(app, *one)).collect();
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
            let row = Row::new(ctx.tokens.md, section.label(), style);
            let row = match counts.get(at).filter(|one| !one.is_empty()) {
                Some(count) => row.end(count, ctx.styles.small(Role::Attention)),
                None => row,
            };
            row.target(Target::SettingsSection(*section))
        })
        .collect();
    glass_list(ctx, list, &rows, up);
}

/// How many things a section still needs, as its row shows it; empty for none.
fn needs(app: &AppState, section: Section) -> String {
    let count = match section {
        Section::Setup => rows::setup_left(app),
        Section::Providers => usize::from(rows::no_source(app)) + rows::source_gaps(app),
        _ => 0,
    };
    match count {
        0 => String::new(),
        n => n.to_string(),
    }
}

fn searched(ctx: &mut Ctx, line: Rect, settings: &SettingsUi) {
    let field = glass_field(ctx, line);
    let search = Search::new(&settings.search, Target::SettingsSearch, settings.typing);
    search.hint("search a setting").glass(true).draw(ctx, field);
}

/// Where the sign-in's terminal stands: the lower half of the form.
pub(crate) fn login_pane(window: Rect, tokens: &Tokens) -> Rect {
    let [_, form] = row_in(
        window,
        [Spec::default().width(tokens.sidebar), Spec::fill()],
    );
    let [_, lower] = column_in(
        form.pad(Edges::all(tokens.md)),
        [Spec::fill(), Spec::fill()],
    );
    lower
}

/// The title, then what takes the window back.
fn back(ctx: &mut Ctx, line: Rect) {
    let md = ctx.tokens.md;
    let mut room = line.pad(Edges::across(md, md));
    let word = Button::glass("back · esc", Target::SettingsBack, Role::Muted, &ctx.styles);
    word.right(ctx, &mut room, 0.0);
    Label::new("Settings", ctx.styles.title(Role::Text)).draw(ctx, room);
}
