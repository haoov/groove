//! The window while the modes layout is on: the rail, the session's bar of title and tabs, the
//! middle under it with the terminals' drawer at its foot, and the right sidebar.

use groove_gfx::{Edges, Rect};

use super::{Folds, Layout};
use crate::Ui;
use groove_ui_kit::base::tokens::Tokens;

impl Layout {
    /// The board right of the rail cut into the bar, the middle, the drawer and the sidebar; the
    /// strip and its parts empty.
    pub(super) fn moded(mut self, tokens: &Tokens, ui: &Ui, folds: Folds) -> Self {
        let (sm, bar) = (tokens.sm, tokens.bar);
        let (board, window) = (self.board, self.window);
        let side = (ui.split.sidebar * tokens.scale).floor().min(board.w);
        // The sidebar's place at the window's right edge, its window inside as the rail's are.
        let place = Rect::new(window.right() - side, board.y, side, board.h);
        let column = place.pad(Edges::all(sm));
        let open = !ui.session.folded;
        let high = if open { column.h } else { bar.min(column.h) };
        self.sidebar = Rect::new(column.x, column.y, column.w, high);
        let left = match folds.sessions && folds.feed {
            true => self.sessions.x,
            false => board.x,
        };
        self.top = Rect::new(board.x, column.y, (column.x - sm - board.x).max(0.0), bar);
        let right = match open {
            true => column.x - sm,
            false => column.right(),
        };
        let floor = column.bottom();
        let drawer = match (folds.manual, ui.session.terminals) {
            (true, _) => (ui.split.manual * tokens.scale).floor(),
            (false, true) => bar,
            (false, false) => 0.0,
        };
        let drawer = drawer.min((floor - self.top.bottom() - sm * 2.0).max(0.0));
        self.manual = Rect::new(board.x, floor - drawer, (right - board.x).max(0.0), drawer);
        let y = self.top.bottom() + sm;
        let gap = if drawer > 0.0 { sm } else { 0.0 };
        let mut foot = self.manual.y - gap;
        if left < board.x {
            foot = foot.min(self.feed.y - sm);
        }
        let tall = (foot - y).max(0.0);
        self.agent = Rect::new(left, y, (right - left).max(0.0), tall);
        let none = Rect::new(window.w, window.h, 0.0, 0.0);
        (self.strip, self.header, self.info) = (none, none, none);
        (self.workspace, self.work, self.alerts) = (none, none, none);
        self
    }
}
