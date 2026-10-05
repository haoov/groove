//! The window's four columns, full height: the rail, the agent, the workspace, the sidebar.

use groove_gfx::{CellSize, Rect, Size};

use crate::Ui;
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::layout::{Boxes, Spec};

mod split;

pub use split::{Edge, Split};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub window: Rect,
    pub rail: Rect,
    pub agent: Rect,
    /// The workspace's two first lines: the title, then what it points at.
    pub header: Rect,
    /// Under the header: the tabs and the tab.
    pub workspace: Rect,
    /// The row of the agent's own actions, under its screen.
    pub agent_bar: Rect,
    /// The tab's own list, folded to nothing when the tab has none.
    pub sidebar: Rect,
    /// The commit box at the sidebar's foot, as tall as the user has dragged it.
    pub commit: Rect,
    /// The rail's feed, above its footer, as tall as the user has dragged it.
    pub feed: Rect,
    /// The terminals under the workspace's tab: their bar alone while folded.
    pub manual: Rect,
    /// Everything the board takes: the window but the rail.
    pub board: Rect,
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens, split: Split, sidebar: bool) -> Self {
        let window = size.rect();
        let scale = |logical: f32| (logical * tokens.scale).floor();
        let fill = Spec::default().grow(1.0);
        let tall = |height: f32| Spec::default().height(height);
        let mut boxes = Boxes::new();
        let sessions = boxes.leaf(fill);
        let band = (scale(split.feed).min(window.h) - tokens.bar).max(tokens.row);
        let feed = boxes.leaf(tall(band));
        let footer = boxes.leaf(tall(tokens.bar));
        let rail = Spec::default().width(scale(split.rail));
        let rail = boxes.column(rail, &[sessions, feed, footer]);
        let screen = boxes.leaf(fill);
        let agent_bar = boxes.leaf(tall(tokens.bar));
        let agent = Spec::default().width(scale(split.agent));
        let agent = boxes.column(agent, &[screen, agent_bar]);
        let header = boxes.leaf(tall(tokens.header + tokens.row + tokens.sm));
        let workspace = boxes.leaf(fill);
        let work = boxes.column(fill, &[header, workspace]);
        let list = boxes.leaf(fill);
        let commit = boxes.leaf(tall(scale(split.commit).min(window.h)));
        let aside = Spec::default().width(scale(split.aside(sidebar)));
        let sidebar = boxes.column(aside, &[list, commit]);
        let board = boxes.row(fill, &[agent, work, sidebar]);
        let root = boxes.row(Spec::default(), &[rail, board]);
        boxes.place(root, window);
        let work = boxes.rect(work);
        Self {
            window,
            feed: boxes.rect(feed),
            commit: boxes.rect(commit),
            rail: boxes.rect(rail),
            agent: boxes.rect(agent),
            agent_bar: boxes.rect(agent_bar),
            header: boxes.rect(header),
            workspace: boxes.rect(workspace),
            sidebar: boxes.rect(sidebar),
            board: boxes.rect(board),
            manual: Rect::new(work.x, window.h, work.w, 0.0),
        }
    }

    pub fn of(metrics: Metrics, ui: &Ui) -> Self {
        let tokens = metrics.tokens();
        let mut held = Self::new(metrics.size, &tokens, ui.split, ui.session.sidebar());
        if ui.rail.folded {
            let row = tokens.row;
            held.feed = Rect::new(0.0, held.feed.bottom() - row, held.feed.w, row);
        }
        held = held.committing(ui.session.commits());
        let tall = match ui.session.manual {
            true => (ui.split.manual * tokens.scale).floor(),
            false => tokens.bar,
        };
        held.manual = held.workspace.take_bottom(tall.min(held.workspace.h));
        match ui.session.alone {
            true => held.alone(),
            false => held,
        }
    }

    /// The agent pane over everything right of the rail; the workspace's parts empty.
    pub fn alone(mut self) -> Self {
        let (rail, window) = (self.rail.w, self.window);
        self.agent = Rect::new(rail, 0.0, (window.w - rail).max(0.0), window.h);
        self.agent_bar = Rect::new(rail, self.agent_bar.y, self.agent.w, self.agent_bar.h);
        let none = Rect::new(window.w, window.h, 0.0, 0.0);
        (self.header, self.workspace, self.sidebar) = (none, none, none);
        (self.commit, self.manual) = (none, none);
        self
    }

    /// The same, with no commit box under the sidebar unless `shown`.
    pub fn committing(mut self, shown: bool) -> Self {
        if !shown {
            self.commit = Rect::new(self.commit.x, self.window.h, self.commit.w, 0.0);
        }
        self
    }

    /// Where the agent's grid starts inside its pane, under the bar.
    pub fn agent_origin(&self, tokens: &Tokens) -> (f32, f32) {
        (self.agent.x + tokens.sm, self.agent.y + tokens.sm)
    }

    /// The columns and rows the agent pane holds at this cell size.
    pub fn agent_grid(&self, tokens: &Tokens, cell: CellSize) -> (u16, u16) {
        let screen = Rect {
            h: self.agent.h - self.agent_bar.h,
            ..self.agent
        };
        grid_in(screen, tokens, cell)
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
