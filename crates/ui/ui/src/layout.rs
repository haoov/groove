//! The window: the rail's two windows, the session's bar, its middle, its terminals and its sidebar.

use groove_gfx::{CellSize, Edges, Rect, Size};

use crate::Ui;
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::layout::{Boxes, Node, Spec};

mod folds;
mod modes;
mod split;

pub use folds::Folds;
pub use split::{Edge, Split};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub window: Rect,
    /// The rail's place, its two windows of glass inside it.
    pub rail: Rect,
    /// The sessions' window: the Board row and the sessions, down to a gap above the feed.
    pub sessions: Rect,
    /// The session's middle, under its bar: the mode shown, across the rail's column too while
    /// both its windows are folded.
    pub agent: Rect,
    /// The session's sidebar at the window's right edge: its head alone while folded.
    pub sidebar: Rect,
    /// The feed's window under the sessions', as tall as the user has dragged it: its header alone while folded.
    pub feed: Rect,
    /// The terminals at the middle's foot: their bar alone while folded.
    pub manual: Rect,
    /// Everything the board takes: the window but the rail.
    pub board: Rect,
    /// The board's columns: right of the rail, clear of the bar's room above and below.
    pub lists: Rect,
    /// The session's bar: its title and its tabs.
    pub top: Rect,
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens, split: Split) -> Self {
        let window = size.rect();
        let mut boxes = Boxes::new();
        let rail = rail(&mut boxes, tokens, split);
        let board = boxes.leaf(Spec::default().grow(1.0));
        let root = boxes.row(Spec::default(), &[rail, board]);
        boxes.place(root, window);
        let place = boxes.rect(rail);
        let (sessions, feed) = rail_windows(place, tokens, (split.feed * tokens.scale).floor());
        let board = boxes.rect(board);
        let none = Rect::new(window.w, window.h, 0.0, 0.0);
        Self {
            window,
            sessions,
            feed,
            rail: place,
            agent: none,
            sidebar: none,
            board,
            lists: board.pad(Edges::xy(0.0, tokens.bar + tokens.sm)),
            manual: none,
            top: none,
        }
    }

    /// Where everything stands with these folds.
    fn folded(metrics: Metrics, ui: &Ui, folds: Folds) -> Self {
        let tokens = metrics.tokens();
        let held = Self::new(metrics.size, &tokens, ui.split);
        let mut held = held.rail_folds(&tokens, folds).moded(&tokens, ui, folds);
        if folds.sessions && folds.feed {
            let grow = (held.lists.x - held.window.x).max(0.0);
            let lists = held.lists;
            held.lists = Rect::new(held.window.x, lists.y, lists.w + grow, lists.h);
        }
        held
    }

    /// The manual section's grids under its bar: `count` side by side, a hairline between.
    pub fn shell_panes(&self, tokens: &Tokens, count: usize) -> Vec<Rect> {
        let mut body = self.manual;
        body.take_top(tokens.bar);
        let count = count.max(1);
        let rules = tokens.hairline * (count - 1) as f32;
        let wide = ((body.w - rules) / count as f32).floor();
        let mut panes: Vec<Rect> = (1..count)
            .map(|_| {
                let pane = body.take_left(wide);
                body.take_left(tokens.hairline);
                pane
            })
            .collect();
        panes.push(body);
        panes
    }
}

/// The rail's place: a column as wide as the user has dragged it.
fn rail(boxes: &mut Boxes, tokens: &Tokens, split: Split) -> Node {
    boxes.leaf(Spec::default().width((split.rail * tokens.scale).floor()))
}

/// The two windows inside the rail's place, a small gap from its edges and from each other:
/// the feed's top at the line the user dragged, `feed` from the window's foot.
fn rail_windows(place: Rect, tokens: &Tokens, feed: f32) -> (Rect, Rect) {
    let (sm, row) = (tokens.sm, tokens.row);
    let column = place.pad(Edges::all(sm));
    let lowest = column.y + row * 2.0 + sm;
    let top = (place.bottom() - feed).clamp(lowest.min(column.bottom()), column.bottom());
    let feed = Rect::new(column.x, top, column.w, (column.bottom() - top).max(0.0));
    let sessions = Rect::new(column.x, column.y, column.w, (top - sm - column.y).max(0.0));
    (sessions, feed)
}

/// The columns and rows a terminal holds in `rect`, its padding kept, at this cell size.
pub fn grid_in(rect: Rect, tokens: &Tokens, cell: CellSize) -> (u16, u16) {
    let pad = tokens.sm * 2.0;
    let cols = ((rect.w - pad) / cell.width).floor().max(1.0);
    let rows = ((rect.h - pad) / cell.height).floor().max(1.0);
    (cols as u16, rows as u16)
}

/// The cell of a grid that starts at `origin` the point stands on.
pub fn cell_at(origin: (f32, f32), cell: CellSize, point: (f32, f32)) -> (usize, usize) {
    let col = ((point.0 - origin.0) / cell.width).floor().max(0.0) as usize;
    let row = ((point.1 - origin.1) / cell.height).floor().max(0.0) as usize;
    (col, row)
}
