use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor};
use groove_types::{AnsiPalette, Screen, ScreenCell};

use crate::color::resolve;

/// The visible grid as cells with resolved colours.
pub(crate) fn snapshot<L: EventListener>(term: &Term<L>, palette: &AnsiPalette) -> Screen {
    let content = term.renderable_content();
    let (cols, rows) = (term.columns(), term.screen_lines());
    let mut cells = vec![blank(palette); cols * rows];
    for indexed in content.display_iter {
        let (col, row) = (indexed.point.column.0, indexed.point.line.0);
        if row < 0 {
            continue;
        }
        if let Some(slot) = cells.get_mut(row as usize * cols + col) {
            *slot = convert(indexed.cell, content.colors, palette);
        }
    }
    let shown =
        content.mode.contains(TermMode::SHOW_CURSOR) && content.cursor.shape != CursorShape::Hidden;
    let cursor = shown.then(|| {
        let p = content.cursor.point;
        (p.column.0, p.line.0.max(0) as usize)
    });
    Screen {
        cols,
        rows,
        cells,
        cursor,
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
