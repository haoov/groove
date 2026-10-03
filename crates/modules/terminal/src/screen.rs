//! The grid as the ui takes it: cells, colours, the cursor.

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use groove_types::{AnsiPalette, Screen, ScreenCell};

use crate::color::resolve;

/// The rows a selection covers, each with the columns it takes.
fn spans(
    held: Option<SelectionRange>,
    scrolled: i32,
    cols: usize,
    rows: usize,
) -> Vec<groove_types::Selected> {
    let Some(held) = held else {
        return Vec::new();
    };
    let (top, foot) = (held.start.line.0 + scrolled, held.end.line.0 + scrolled);
    (top.max(0)..=foot.min(rows as i32 - 1))
        .map(|row| {
            let from = match row == top {
                true => held.start.column.0,
                false => 0,
            };
            let to = match row == foot {
                true => (held.end.column.0 + 1).min(cols),
                false => cols,
            };
            groove_types::Selected {
                row: row as usize,
                from: from.min(to),
                to,
            }
        })
        .collect()
}

/// The visible grid as cells with resolved colours.
pub(crate) fn snapshot<L: EventListener>(term: &Term<L>, palette: &AnsiPalette) -> Screen {
    let content = term.renderable_content();
    let (cols, rows) = (term.columns(), term.screen_lines());
    let scrolled = content.display_offset as i32;
    let mut cells = vec![blank(palette); cols * rows];
    for indexed in content.display_iter {
        let col = indexed.point.column.0;
        let row = indexed.point.line.0 + scrolled;
        if row < 0 {
            continue;
        }
        if let Some(slot) = cells.get_mut(row as usize * cols + col) {
            *slot = convert(indexed.cell, content.colors, palette);
        }
    }
    let shown =
        content.mode.contains(TermMode::SHOW_CURSOR) && content.cursor.shape != CursorShape::Hidden;
    let cursor = shown
        .then(|| {
            let p = content.cursor.point;
            usize::try_from(p.line.0 + scrolled)
                .ok()
                .map(|row| (p.column.0, row))
        })
        .flatten()
        .filter(|(_, row)| *row < rows);
    Screen {
        cols,
        rows,
        cells,
        cursor,
        selected: spans(content.selection, scrolled, cols, rows),
    }
}

fn blank(palette: &AnsiPalette) -> ScreenCell {
    ScreenCell {
        ch: ' ',
        fg: palette.foreground,
        bg: None,
        bold: false,
        spacer: false,
    }
}

fn convert(cell: &Cell, overrides: &Colors, palette: &AnsiPalette) -> ScreenCell {
    let flags = cell.flags;
    let mut fg = resolve(cell.fg, overrides, palette);
    let mut bg = match cell.bg {
        Color::Named(NamedColor::Background) => None,
        other => Some(resolve(other, overrides, palette)),
    };
    if flags.contains(Flags::INVERSE) {
        let lit = fg;
        fg = bg.unwrap_or(palette.background);
        bg = Some(lit);
    }
    if flags.contains(Flags::DIM) {
        fg = fg.dimmed();
    }
    if flags.contains(Flags::HIDDEN) {
        fg = bg.unwrap_or(palette.background);
    }
    let spacer = flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER);
    ScreenCell {
        ch: if spacer { ' ' } else { cell.c },
        fg,
        bg,
        bold: flags.contains(Flags::BOLD),
        spacer,
    }
}
