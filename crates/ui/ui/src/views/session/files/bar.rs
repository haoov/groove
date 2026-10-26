//! The search at the sidebar's top: one rounded field, for a path or for text, always there.

use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Bar, Term};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::{Panel, band};
use groove_ui_kit::widgets::{Button, Search};

/// The field for the term in use, and what switches it to the other. Returns what it took.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, ui: &Ui) -> Rect {
    let (row, sm) = (ctx.tokens.row, ctx.tokens.sm);
    let [whole, _] = column_in(rect, [Spec::default().height(row + sm), Spec::fill()]);
    let field = band(whole, sm / 2.0, row).pad(Edges::across(sm, 0.0));
    let ground = ctx.styles.glass_hover();
    Panel::default().ground(ground).radius(sm).draw(ctx, field);
    let bar = &ui.session.bar;
    let term = shown(bar);
    let other = match term {
        Term::Path => Term::Text,
        Term::Text => Term::Path,
    };
    let mut room = field;
    let switch = Button::new(other.label(), Target::Term(other), Role::Faint, ground);
    switch
        .flat()
        .hover(ctx.styles.glass_hover())
        .right(ctx, &mut room, 0.0);
    let held = match term {
        Term::Path => &bar.path,
        Term::Text => &bar.text,
    };
    let search = Search::new(held, Target::Term(term), bar.typing == Some(term));
    search.code().glass(true).faint(Role::Faint).draw(ctx, room);
    whole
}

/// The term the field shows: the one typed in, else the one holding a query, else the path.
pub(crate) fn shown(bar: &Bar) -> Term {
    match bar.typing {
        Some(term) => term,
        None if !bar.text.is_empty() && bar.path.is_empty() => Term::Text,
        None => Term::Path,
    }
}
