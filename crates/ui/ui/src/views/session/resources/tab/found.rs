//! An Application's resources find bar, under the filters while open or holding words.

use groove_gfx::Rect;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::widgets::{Search, glass_field};

use super::super::opened::Opened;
use crate::ctx::Ctx;
use crate::hit::Target;

/// The bar drawn at the top of `body` when it stands; the room left under it.
pub(super) fn bar(ctx: &mut Ctx, body: Rect, tab: &Opened) -> Rect {
    let argo = &tab.argo;
    if !argo.finding && argo.search.is_empty() {
        return body;
    }
    let [line, rest] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    let field = glass_field(ctx, line);
    Search::new(&argo.search, Target::ArgoSearch, argo.typing)
        .hint("name")
        .code()
        .faint(Role::Faint)
        .draw(ctx, field);
    rest
}
