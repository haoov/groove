//! The bar over an object's YAML: EDIT, or APPLY and DISCARD, then what the draft stands at.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::FollowKey;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Button;

use crate::ctx::Ctx;
use crate::hit::Target;

pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, key: &FollowKey) {
    let yamls = &app.cluster.store.follows.yamls;
    let (gap, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md, md));
    let tint = ctx.styles.band_tint(Role::Attention);
    let edge = ctx.styles.band_edge(Role::Attention);
    let lead = |word, target| Button::new(word, target, Role::Attention, tint).edged(edge);
    let Some(draft) = yamls.draft(key) else {
        lead("EDIT", Target::YamlWrite(true)).left(ctx, &mut room, gap);
        return;
    };
    lead("APPLY", Target::YamlApply).left(ctx, &mut room, gap);
    let raised = ctx.styles.panel_head();
    let discard = Button::new("DISCARD", Target::YamlWrite(false), Role::Muted, raised);
    discard.left(ctx, &mut room, md);
    let (said, role) = match (yamls.moved(key), draft.edited()) {
        (true, _) => ("the object changed since you started editing", Role::Warn),
        (false, true) => ("edited", Role::Attention),
        (false, false) => ("nothing changed yet", Role::Faint),
    };
    Label::new(said, ctx.styles.small(role)).draw(ctx, room);
}
