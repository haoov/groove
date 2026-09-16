//! The band above the rows: the path, what it changed, and the three views.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::DiffView;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::views::session::components::files;
use crate::widget::{elide_start, hairline, row};

/// The file's path, what it changed, and which view it is drawn in.
pub(super) fn draw(ctx: &mut Ctx, band: Rect, app: &AppState, ui: &Ui, path: &str) {
    let rule = ctx.styles.line();
    let switched = switch(ctx, band, ui.session.view);
    let left = Rect::new(band.x, band.y, switched - band.x, band.h);
    let counts = counted(app, path);
    let at = counts
        .map(|counts| written(ctx, left, counts))
        .unwrap_or(left.right());
    let style = ctx.styles.code(Role::Muted);
    let room = (at - band.x - ctx.tokens.md * 2.0).max(0.0);
    let text = elide_start(ctx, path, &style, room);
    row(
        ctx,
        Rect::new(band.x, band.y, at - band.x, band.h),
        ctx.tokens.md,
        &text,
        style,
    );
    hairline(ctx, band, rule);
}

/// The three views, the current one raised. Returns where they start.
fn switch(ctx: &mut Ctx, line: Rect, current: DiffView) -> f32 {
    let (pad, gap) = (ctx.tokens.sm, ctx.tokens.xs);
    let mut at = line.right();
    for view in DiffView::ALL.into_iter().rev() {
        let role = match view == current {
            true => Role::Text,
            false => Role::Faint,
        };
        let style = ctx.styles.small(role);
        let width = ctx.measure(view.label(), &style) + pad * 2.0;
        at -= width;
        let box_ = Rect::new(at, line.y, width, line.h);
        if view == current {
            let raised = ctx.styles.raised();
            ctx.quad(box_, raised);
        }
        row(ctx, box_, pad, view.label(), style);
        ctx.hit(box_, Target::View(view));
        at -= gap;
    }
    at
}

/// What the file added and deleted, right of its path. Returns where they start.
fn written(ctx: &mut Ctx, line: Rect, counts: (u32, u32)) -> f32 {
    let mut at = line.right() - ctx.tokens.md;
    for (count, role, sign) in [(counts.1, Role::Bad, '-'), (counts.0, Role::Ok, '+')] {
        if count == 0 {
            continue;
        }
        let style = ctx.styles.code(role);
        let text = format!("{sign}{count}");
        let width = ctx.measure(&text, &style);
        at -= width;
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// The summary's counts for this path.
fn counted(app: &AppState, path: &str) -> Option<(u32, u32)> {
    files::changed(app)
        .iter()
        .find(|file| file.path == path)
        .map(|file| (file.added, file.deleted))
}
