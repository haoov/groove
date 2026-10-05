//! How big a text leaf stands: its widest word, its widest line, and the rows it wraps to.

use groove_gfx::TextStyle;
use taffy::AvailableSpace;

use crate::base::ctx::{App, Ctx};
use crate::text::wrapped;

/// What a text leaf holds for the layout to measure, and what one solve already measured.
#[derive(Debug, Clone)]
pub(super) struct Text {
    text: String,
    style: TextStyle,
    line: f32,
    widest_word: Option<f32>,
    widest_line: Option<f32>,
    /// The last width wrapped, and how many rows it took.
    rows: Option<(f32, usize)>,
}

impl Text {
    pub(super) fn new(text: &str, style: TextStyle, line: f32) -> Self {
        Self {
            text: text.to_string(),
            style,
            line,
            widest_word: None,
            widest_line: None,
            rows: None,
        }
    }
}

/// The size a text leaf takes: as wide as its longest line or the room, as tall as its rows.
pub(super) fn measure<A: App>(
    ctx: &mut Ctx<'_, A>,
    text: &mut Text,
    known: taffy::Size<Option<f32>>,
    room: taffy::Size<AvailableSpace>,
) -> taffy::Size<f32> {
    let width = known.width.unwrap_or_else(|| match room.width {
        AvailableSpace::MinContent => widest_word(ctx, text),
        AvailableSpace::MaxContent => widest_line(ctx, text),
        AvailableSpace::Definite(room) => room.min(widest_line(ctx, text)),
    });
    let height = known
        .height
        .unwrap_or_else(|| rows(ctx, text, width) as f32 * text.line);
    taffy::Size { width, height }
}

fn widest_word<A: App>(ctx: &mut Ctx<'_, A>, text: &mut Text) -> f32 {
    *text
        .widest_word
        .get_or_insert_with(|| widest(ctx, text.text.split_whitespace(), &text.style))
}

fn widest_line<A: App>(ctx: &mut Ctx<'_, A>, text: &mut Text) -> f32 {
    *text
        .widest_line
        .get_or_insert_with(|| widest(ctx, text.text.split('\n'), &text.style))
}

fn rows<A: App>(ctx: &mut Ctx<'_, A>, text: &mut Text, width: f32) -> usize {
    match text.rows {
        Some((wrapped_at, rows)) if wrapped_at == width => rows,
        _ => {
            let rows = match ctx.kept_rows(&text.text, &text.style, width) {
                Some(rows) => rows,
                None => {
                    let rows = wrapped(ctx, &text.text, &text.style, width).len();
                    ctx.keep_rows(&text.text, &text.style, width, rows);
                    rows
                }
            };
            text.rows = Some((width, rows));
            rows
        }
    }
}

fn widest<'t, A: App>(
    ctx: &mut Ctx<'_, A>,
    pieces: impl Iterator<Item = &'t str>,
    style: &TextStyle,
) -> f32 {
    pieces
        .map(|piece| ctx.measure(piece.trim(), style))
        .fold(0.0, f32::max)
}
