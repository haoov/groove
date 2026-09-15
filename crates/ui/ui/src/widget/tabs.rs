use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{hairline, row};

/// Labels side by side from the left edge of `rect`, the selected one on a background
/// the width of its text, a hairline under the strip. Returns where each one landed.
pub fn tabs(ctx: &mut Ctx, rect: Rect, labels: &[&str], selected: usize) -> Vec<Rect> {
    let pad = ctx.tokens.md;
    let mut boxes = Vec::with_capacity(labels.len());
    let mut x = rect.x;
    for (i, label) in labels.iter().enumerate() {
        let picked = i == selected;
        let style = ctx
            .styles
            .label(if picked { Role::Text } else { Role::Muted });
        let width = ctx.measure(label, &style) + pad * 2.0;
        let tab = Rect::new(x, rect.y, width, rect.h);
        if picked {
            let raised = ctx.styles.raised();
            ctx.quad(tab, raised);
        }
        row(ctx, tab, pad, label, style);
        boxes.push(tab);
        x += width;
    }
    let line = ctx.styles.line();
    hairline(ctx, rect, line);
    boxes
}
