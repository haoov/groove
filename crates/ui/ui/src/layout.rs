//! The window's four columns, full height: the rail, the agent, the workspace, the sidebar.

use groove_gfx::{CellSize, Rect, Size};
use groove_types::Panes;

use crate::Ui;
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::{
    AGENT_MIN, CODE_MIN, COMMIT_MIN, FEED_MIN, FILES_MIN, MANUAL_MIN, MANUAL_TALL, MESSAGE_LINES,
    RAIL_MIN, SESSIONS_MIN, SIDEBAR_MIN, Tokens, WORKSPACE_MIN,
};

/// A boundary the user drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Between the rail and the agent pane.
    Rail,
    /// Between the agent pane and the workspace.
    Agent,
    /// Between the workspace and the sidebar.
    Sidebar,
    /// Between the changed files and the commit box under them.
    Commit,
    /// Between the rail's own rows and the feed under them.
    Feed,
    /// Between the workspace's tab and the manual section under it.
    Manual,
}

impl Edge {
    /// Every boundary: the three columns, then the ones across the sidebar and the workspace.
    pub const ALL: [Edge; 5] = [
        Edge::Rail,
        Edge::Agent,
        Edge::Sidebar,
        Edge::Commit,
        Edge::Manual,
    ];

    /// Whether the boundary is a vertical line, which the pointer moves sideways.
    pub fn upright(self) -> bool {
        !matches!(self, Edge::Commit | Edge::Feed | Edge::Manual)
    }
}

/// Every column's width but the workspace's, which takes the rest, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    /// How tall the commit box stands at the sidebar's foot.
    pub commit: f32,
    /// How tall the rail's feed stands, the footer under it included.
    pub feed: f32,
    /// How tall the manual section stands open.
    pub manual: f32,
}

impl Default for Split {
    fn default() -> Self {
        let tokens = Tokens::default();
        Self {
            rail: tokens.rail,
            agent: tokens.agent,
            sidebar: tokens.sidebar,
            commit: tokens.row + tokens.line * MESSAGE_LINES as f32,
            feed: FEED_MIN * 2.0,
            manual: MANUAL_TALL,
        }
    }
}

impl Split {
    /// What a past run left, no column under its minimum.
    pub fn of(panes: Panes) -> Self {
        Self {
            rail: panes.rail.max(RAIL_MIN),
            agent: panes.agent.max(AGENT_MIN),
            sidebar: panes.sidebar.max(SIDEBAR_MIN),
            commit: panes.commit.max(COMMIT_MIN),
            feed: panes.feed.max(FEED_MIN),
            manual: panes.manual.max(MANUAL_MIN),
        }
    }

    pub fn panes(&self) -> Panes {
        Panes {
            rail: self.rail,
            agent: self.agent,
            sidebar: self.sidebar,
            commit: self.commit,
            feed: self.feed,
            manual: self.manual,
        }
    }

    /// Puts `edge` at `x`, moving only its two columns and none under its minimum.
    pub fn drag(&mut self, edge: Edge, at: f32, window: (f32, f32), sidebar: bool) {
        let (width, height) = window;
        let x = at;
        match edge {
            Edge::Rail => {
                let held = self.rail + self.agent;
                self.rail = x.clamp(RAIL_MIN, (held - AGENT_MIN).max(RAIL_MIN));
                self.agent = held - self.rail;
            }
            Edge::Agent => {
                let most = (self.room(width, sidebar) - WORKSPACE_MIN).max(AGENT_MIN);
                self.agent = (x - self.rail).clamp(AGENT_MIN, most);
            }
            Edge::Sidebar => {
                let most = (width - self.rail - self.agent - WORKSPACE_MIN).max(SIDEBAR_MIN);
                self.sidebar = (width - x).clamp(SIDEBAR_MIN, most);
            }
            Edge::Commit => {
                let most = (height - FILES_MIN).max(COMMIT_MIN);
                self.commit = (height - at).clamp(COMMIT_MIN, most);
            }
            Edge::Feed => {
                let most = (height - SESSIONS_MIN).max(FEED_MIN);
                self.feed = (height - at).clamp(FEED_MIN, most);
            }
            Edge::Manual => {
                let most = (height - CODE_MIN).max(MANUAL_MIN);
                self.manual = (height - at).clamp(MANUAL_MIN, most);
            }
        }
    }

    pub fn edge_at(&self, edge: Edge, window: (f32, f32), sidebar: bool) -> f32 {
        let (width, height) = window;
        match edge {
            Edge::Rail => self.rail,
            Edge::Agent => self.rail + self.agent,
            Edge::Sidebar => width - self.aside(sidebar),
            Edge::Commit => height - self.commit,
            Edge::Feed => height - self.feed,
            Edge::Manual => height - self.manual,
        }
    }

    fn room(&self, width: f32, sidebar: bool) -> f32 {
        (width - self.rail - self.aside(sidebar)).max(1.0)
    }

    fn aside(&self, sidebar: bool) -> f32 {
        match sidebar {
            true => self.sidebar,
            false => 0.0,
        }
    }
}

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
        let rail = scale(split.rail);
        let agent = scale(split.agent);
        let aside = scale(split.aside(sidebar));
        let work_x = rail + agent;
        let work_width = (window.w - work_x - aside).max(0.0);
        let box_ = scale(split.commit).min(window.h);
        let foot = tokens.bar;
        let band = (scale(split.feed).min(window.h) - foot).max(tokens.row);
        let head = tokens.header + tokens.row + tokens.sm;
        Self {
            window,
            feed: Rect::new(0.0, window.h - foot - band, rail, band),
            commit: Rect::new(work_x + work_width, window.h - box_, aside, box_),
            rail: Rect::new(0.0, 0.0, rail, window.h),
            agent: Rect::new(rail, 0.0, agent, window.h),
            agent_bar: Rect::new(rail, window.h - tokens.bar, agent, tokens.bar),
            header: Rect::new(work_x, 0.0, work_width, head),
            workspace: Rect::new(work_x, head, work_width, window.h - head),
            sidebar: Rect::new(work_x + work_width, 0.0, aside, window.h),
            board: Rect::new(rail, 0.0, (window.w - rail).max(0.0), window.h),
            manual: Rect::new(work_x, window.h, work_width, 0.0),
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
        held
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
