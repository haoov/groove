use groove_gfx::{Align, Color, Edges, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::shape::{after_mark, hoverable, leading};
use crate::text::{elide, lit_row, row};
use crate::widgets::inset_row;

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
    /// Characters of the text drawn in that style, such as what a query matched.
    pub lit: Option<(&'a [std::ops::Range<usize>], TextStyle)>,
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
            lit: None,
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

    pub fn lit(mut self, ranges: &'a [std::ops::Range<usize>], style: TextStyle) -> Self {
        self.lit = Some((ranges, style));
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
        drawn(ctx, (rect, line), item);
        y += height;
    }
    y
}

/// Rows on glass, each ground rounded and see-through; returns the y under the last.
pub fn glass_list<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    rows: &[Row<'_, A::Target>],
    selected: Option<usize>,
) -> f32 {
    let height = ctx.tokens.glass_row;
    let mut y = rect.y;
    for (i, item) in rows.iter().enumerate() {
        let line = Rect::new(rect.x, y, rect.w, height);
        let row = match &item.target {
            Some(target) => inset_row(ctx, line, target.clone(), (selected == Some(i), None)),
            None => line.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm)),
        };
        drawn(ctx, (rect, row), item);
        y += height;
    }
    y
}

fn drawn<A: App>(ctx: &mut Ctx<'_, A>, (rect, line): (Rect, Rect), item: &Row<'_, A::Target>) {
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
    texted(ctx, (line, indent), &text, item);
    if let Some((at, text, style)) = item.aside {
        let room = (right - (rect.x + at)).max(0.0);
        let aside = Rect::new(rect.x + at, line.y, room, line.h);
        let text = elide(ctx, text, &style, room);
        row(ctx, aside, 0.0, &text, style);
    }
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

/// The row's text, its lit characters kept where the cut leaves them.
fn texted<T>(ctx: &mut Ctx<'_, impl App>, at: (Rect, f32), text: &str, item: &Row<'_, T>) {
    let Some((ranges, lit)) = item.lit else {
        return row(ctx, at.0, at.1, text, item.style);
    };
    let whole = text.chars().count() - usize::from(text != item.text);
    let kept: Vec<_> = ranges.iter().filter(|r| r.end <= whole).cloned().collect();
    lit_row(ctx, at, text, item.style, (&kept, lit));
}
