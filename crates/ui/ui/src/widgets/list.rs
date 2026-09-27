use groove_gfx::{Color, Rect, TextStyle};

use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::shape::{after_mark, leading};
use crate::text::{elide, row};

/// One line of a list: a text at an indent, and a second text at a fixed offset.
pub struct Row<'a> {
    pub indent: f32,
    pub text: &'a str,
    pub style: TextStyle,
    pub aside: Option<(f32, &'a str, TextStyle)>,
    pub background: Option<Color>,
    /// A mark before the text, in the text's own colour, and its rotation.
    pub mark: Option<(Mark, u8)>,
    /// What a click on the row means.
    pub target: Option<Target>,
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
            target: None,
        }
    }

    /// What a click on this row acts on.
    pub fn target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some((mark, 0));
        self
    }

    pub fn aside(mut self, at: f32, text: &'a str, style: TextStyle) -> Self {
        self.aside = Some((at, text, style));
        self
    }
}

/// Rows from the top of `rect`, the selected one on a ground; returns the y under the last.
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
            let box_ = leading(ctx, line, line.x + indent);
            ctx.icon(box_, mark, turn, item.style.color);
            indent = after_mark(ctx, indent);
        }
        let start = line.x + indent;
        let ends = match item.aside {
            Some((at, _, _)) => rect.x + at - ctx.tokens.sm,
            None => line.right() - ctx.tokens.md,
        };
        let text = elide(ctx, item.text, &item.style, (ends - start).max(0.0));
        row(ctx, line, indent, &text, item.style);
        if let Some((at, text, style)) = item.aside {
            let room = (rect.right() - ctx.tokens.md - (rect.x + at)).max(0.0);
            let aside = Rect::new(rect.x + at, y, room, height);
            let text = elide(ctx, text, &style, room);
            row(ctx, aside, 0.0, &text, style);
        }
        if let Some(target) = &item.target {
            ctx.hit(line, target.clone());
        }
        y += height;
    }
    y
}
