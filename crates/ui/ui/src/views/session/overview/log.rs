//! The overview's log: what the session and its children did, the newest entry first.

use groove_gfx::{Edges, Rect};
use groove_types::LogEntry;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::shape::band;
use groove_ui_kit::widgets::Button;
use groove_ui_kit::text::{Label, ago};

use crate::ctx::Ctx;
use crate::hit::Target;

/// What was done, its age and remove button; why under it; the child and the MR last.
pub(super) fn entry(ctx: &mut Ctx, rect: Rect, one: &LogEntry) {
    let (md, row) = (ctx.tokens.md, ctx.tokens.row);
    let inner = rect.pad(Edges::across(md * 2.0, md * 2.0));
    let mut at = ctx.tokens.xs;
    let mut head = band(inner, at, row);
    let remove = Button::icon(Mark::Close, 0, Target::LogRemove(one.id), Role::Faint);
    remove.right(ctx, &mut head, ctx.tokens.sm);
    let age = ago(one.at.age_at(ctx.now));
    Label::new(&age, ctx.styles.small(Role::Ghost)).right(ctx, &mut head, md);
    Label::new(&one.what, ctx.styles.label(Role::Text)).draw(ctx, head);
    if let Some(why) = &one.why {
        at += row;
        Label::new(why, ctx.styles.body(Role::Muted)).draw(ctx, band(inner, at, row));
    }
    if !footed(one) {
        return;
    }
    at += row;
    let mut foot = band(inner, at, row);
    if let Some(url) = &one.mr_url {
        let link = Label::new(url, ctx.styles.small(Role::Accent));
        let hit = link.right(ctx, &mut foot, md);
        ctx.hit(hit, Target::Link(url.clone()));
    }
    if let Some(child) = &one.from {
        let from = format!("from {child}");
        Label::new(&from, ctx.styles.small(Role::Faint)).draw(ctx, foot);
    }
}

/// How tall one entry stands: a line for what, then one for why and one for its foot when it has them.
pub(super) fn height(ctx: &Ctx, one: &LogEntry) -> f32 {
    let lines = 1 + usize::from(one.why.is_some()) + usize::from(footed(one));
    ctx.tokens.row * lines as f32 + ctx.tokens.xs * 2.0
}

fn footed(one: &LogEntry) -> bool {
    one.from.is_some() || one.mr_url.is_some()
}
