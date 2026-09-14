use groove_gfx::{Color, Rect, TextStyle};

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::widget::{box_in, row};

/// One line of a list: a text at an indent, and a second text at a fixed offset.
pub struct Row<'a> {
    pub indent: f32,
    pub text: &'a str,
    pub style: TextStyle,
    pub aside: Option<(f32, &'a str, TextStyle)>,
    pub background: Option<Color>,
    /// A mark before the text, in the text's own colour, and its rotation.
    pub mark: Option<(Mark, u8)>,
}

impl<'a> Row<'a> {
    pub fn new(indent: f32, text: &'a str, style: TextStyle) -> Self {
        Self {
            indent,
            text,
            style,
            aside: None,
            background: None,
            mark: None,
        }
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some((mark, 0));
        self
    }

    /// A mark that turns, for something in flight.
    pub fn turning(mut self, mark: Mark, turn: u8) -> Self {
        self.mark = Some((mark, turn));
        self
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
        let mut indent = item.indent;
        if let Some((mark, turn)) = item.mark {
            let box_ = box_in(ctx, line, line.x + indent);
            ctx.icon(box_, mark, turn, item.style.color);
            indent += ctx.tokens.icon + ctx.tokens.sm;
        }
        row(ctx, line, indent, item.text, item.style);
        if let Some((at, text, style)) = item.aside {
            let aside = Rect::new(rect.x + at, y, rect.w - at, height);
            row(ctx, aside, 0.0, text, style);
        }
        y += height;
    }
    y
}
