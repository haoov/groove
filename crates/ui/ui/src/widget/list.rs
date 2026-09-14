use groove_gfx::{Color, Rect, TextStyle};

use crate::ctx::Ctx;
use crate::widget::row;

/// One line of a list: a text at an indent, and a second text at a fixed offset.
pub struct Row<'a> {
    pub indent: f32,
    pub text: &'a str,
    pub style: TextStyle,
    pub aside: Option<(f32, &'a str, TextStyle)>,
    pub background: Option<Color>,
}

impl<'a> Row<'a> {
    pub fn new(indent: f32, text: &'a str, style: TextStyle) -> Self {
        Self {
            indent,
            text,
            style,
            aside: None,
            background: None,
        }
    }

    pub fn aside(mut self, at: f32, text: &'a str, style: TextStyle) -> Self {
        self.aside = Some((at, text, style));
        self
    }
}

/// Rows from the top of `rect`, one row height each, the selected one on a background.
/// Returns the y under the last row.
pub fn list(ctx: &mut Ctx, rect: Rect, rows: &[Row<'_>], selected: Option<usize>) -> f32 {
    let height = ctx.tokens.row;
    let raised = ctx.styles.raised();
    let mut y = rect.y;
    for (i, item) in rows.iter().enumerate() {
        let line = Rect::new(rect.x, y, rect.w, height);
        if let Some(color) = item.background.or((selected == Some(i)).then_some(raised)) {
            ctx.quad(line, color);
        }
        row(ctx, line, item.indent, item.text, item.style);
        if let Some((at, text, style)) = item.aside {
            let aside = Rect::new(rect.x + at, y, rect.w - at, height);
            row(ctx, aside, 0.0, text, style);
        }
        y += height;
    }
    y
}
