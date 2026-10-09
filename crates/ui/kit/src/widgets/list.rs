use groove_gfx::{Align, Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::shape::{after_mark, hoverable, leading};
use crate::text::{elide, row};

/// One line of a list: a text at an indent, and a second text at a fixed offset.
pub struct Row<'a, T> {
    pub indent: f32,
    pub text: &'a str,
    pub style: TextStyle,
    pub aside: Option<(f32, &'a str, TextStyle)>,
    /// A text against the right edge, a shortcut beside its action.
    pub end: Option<(&'a str, TextStyle)>,
    pub background: Option<Color>,
    /// A mark before the text and its rotation, in `tint` or the text's own colour.
    pub mark: Option<(Mark, u8)>,
    pub tint: Option<Color>,
    /// The mark drawn this size, centred where a full one stands.
    pub mark_size: Option<f32>,
    /// What a click on the row means.
    pub target: Option<T>,
}

impl<'a, T> Row<'a, T> {
    pub fn new(indent: f32, text: &'a str, style: TextStyle) -> Self {
        Self {
            indent,
            text,
            style,
            aside: None,
            end: None,
            background: None,
            mark: None,
            tint: None,
            mark_size: None,
            target: None,
        }
    }

    /// What a click on this row acts on.
    pub fn target(mut self, target: T) -> Self {
        self.target = Some(target);
        self
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some((mark, 0));
        self
    }

    pub fn tint(mut self, color: Color) -> Self {
        self.tint = Some(color);
        self
    }

    pub fn mark_size(mut self, size: f32) -> Self {
        self.mark_size = Some(size);
        self
    }

    pub fn aside(mut self, at: f32, text: &'a str, style: TextStyle) -> Self {
        self.aside = Some((at, text, style));
        self
    }

    pub fn end(mut self, text: &'a str, style: TextStyle) -> Self {
        self.end = Some((text, style));
        self
    }
}

/// Rows from the top of `rect`, the selected one on a ground; returns the y under the last.
pub fn list<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    rows: &[Row<'_, A::Target>],
    selected: Option<usize>,
) -> f32 {
    let height = ctx.tokens.row;
    let raised = ctx.styles.raised();
    let mut y = rect.y;
    for (i, item) in rows.iter().enumerate() {
        let line = Rect::new(rect.x, y, rect.w, height);
        if let Some(color) = item.background.or((selected == Some(i)).then_some(raised)) {
            ctx.quad(line, color);
        }
        if let Some(target) = &item.target {
            hoverable(ctx, line, target.clone());
        }
        let mut indent = item.indent;
        if let Some((mark, turn)) = item.mark {
            let full = leading(ctx, line, line.x + indent);
            let box_ = match item.mark_size {
                Some(size) => full.align((size, size), Align::Center, Align::Center),
                None => full,
            };
            ctx.icon(box_, mark, turn, item.tint.unwrap_or(item.style.color));
            indent = after_mark(ctx, indent);
        }
        let right = ended(ctx, line, item.end);
        let start = line.x + indent;
        let ends = match item.aside {
            Some((at, _, _)) => rect.x + at - ctx.tokens.sm,
            None => right,
        };
        let text = elide(ctx, item.text, &item.style, (ends - start).max(0.0));
        row(ctx, line, indent, &text, item.style);
        if let Some((at, text, style)) = item.aside {
            let room = (right - (rect.x + at)).max(0.0);
            let aside = Rect::new(rect.x + at, y, room, height);
            let text = elide(ctx, text, &style, room);
            row(ctx, aside, 0.0, &text, style);
        }
        y += height;
    }
    y
}

/// The row's end text against its right edge; returns where the rest must stop.
fn ended<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, end: Option<(&str, TextStyle)>) -> f32 {
    let edge = line.right() - ctx.tokens.md;
    let Some((text, style)) = end else {
        return edge;
    };
    let wide = ctx.measure(text, &style);
    let at = Rect::new(edge - wide, line.y, wide, line.h);
    row(ctx, at, 0.0, text, style);
    edge - wide - ctx.tokens.md
}
