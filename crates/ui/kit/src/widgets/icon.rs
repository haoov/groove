use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;

pub fn icon<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, mark: Mark, role: Role) {
    let color = ctx.styles.color(role);
    ctx.icon(rect, mark, 0, color);
}
