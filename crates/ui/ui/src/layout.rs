//! The window: the rail's two windows, the agent, and the workspace in a strip of glass beside it.

use groove_gfx::{CellSize, Edges, Rect, Size};

use crate::Ui;
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::layout::{Boxes, Node, Spec};

mod split;

pub use split::{Edge, Split};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub window: Rect,
    /// The rail's place, its two windows of glass inside it.
    pub rail: Rect,
    /// The sessions' window: the Board row and the sessions, down to a gap above the feed.
    pub sessions: Rect,
    /// The agent's screen: a bar's room left over it and under it, across the rail's column too
    /// while both its windows are folded.
    pub agent: Rect,
    /// The workspace's pane of glass: its header alone while folded.
    pub strip: Rect,
    /// The strip's first line: the title, the tabs, the actions.
    pub header: Rect,
    /// Under the header: what the tabs point at.
    pub info: Rect,
    /// Under the info line: the tab.
    pub workspace: Rect,
    /// The tab's own list, folded to nothing when the tab has none.
    pub sidebar: Rect,
    /// The feed's window under the sessions', as tall as the user has dragged it: its header alone while folded.
    pub feed: Rect,
    /// The terminals under the workspace's tab: their bar alone while folded.
    pub manual: Rect,
    /// Everything the board takes: the window but the rail.
    pub board: Rect,
    /// The strip open, under its header: the info line, the tab and the sidebar.
    pub work: Rect,
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens, split: Split, sidebar: bool) -> Self {
        let window = size.rect();
        let scale = |logical: f32| (logical * tokens.scale).floor();
        let fill = Spec::default().grow(1.0);
        let tall = |height: f32| Spec::default().height(height);
        let mut boxes = Boxes::new();
        let rail = rail(&mut boxes, tokens, split);
        let screen = boxes.leaf(fill);
        let under = Edges::xy(0.0, tokens.bar + tokens.sm);
        let agent = Spec::default().width(scale(split.agent)).pad(under);
        let agent = boxes.column(agent, &[screen]);
        let header = boxes.leaf(tall(tokens.bar));
        // The info line, a gap over and under it and a hairline between those under it.
        let info = boxes.leaf(tall(tokens.row + tokens.sm + tokens.sm + tokens.sm));
        let workspace = boxes.leaf(fill);
        let aside = Spec::default().width(scale(split.aside(sidebar)));
        let sidebar = boxes.leaf(aside);
        let below = boxes.row(fill, &[workspace, sidebar]);
        let strip = boxes.column(fill.pad(Edges::all(tokens.sm)), &[header, info, below]);
        let board = boxes.row(fill, &[agent, strip]);
        let root = boxes.row(Spec::default(), &[rail, board]);
        boxes.place(root, window);
        let glass = boxes.rect(strip).pad(Edges::all(tokens.sm));
        let header = boxes.rect(header);
        let place = boxes.rect(rail);
        let (sessions, feed) = rail_windows(place, tokens, (split.feed * tokens.scale).floor());
        Self {
            window,
            sessions,
            feed,
            rail: place,
            agent: boxes.rect(agent).pad(under),
            strip: glass,
            header,
            info: boxes.rect(info),
            workspace: boxes.rect(workspace),
            // Inside the sidebar's box, a hairline from its border: its rows reach across it whole.
            sidebar: boxes
                .rect(sidebar)
                .pad(Edges::across(tokens.hairline, tokens.hairline)),
            board: boxes.rect(board),
            work: Rect::new(
                glass.x,
                header.bottom(),
                glass.w,
                glass.bottom() - header.bottom(),
            ),
            manual: Rect::new(glass.x, window.h, glass.w, 0.0),
        }
    }

    pub fn of(metrics: Metrics, ui: &Ui) -> Self {
        let tokens = metrics.tokens();
        let mut held = Self::new(metrics.size, &tokens, ui.split, ui.session.sidebar());
        if ui.rail.folded {
            let feed = held.feed;
            let tall = tokens.bar.min(feed.h);
            held.feed = Rect::new(feed.x, feed.bottom() - tall, feed.w, tall);
            held.sessions.h = (held.feed.y - tokens.sm - held.sessions.y).max(0.0);
        }
        if ui.rail.stowed {
            held = held.sessions_folded(tokens.bar, tokens.sm, ui.rail.folded);
        }
        let asked = ui.examining().is_some() || ui.losing() == Some(&crate::Losing::Everything);
        let stowing = ui.session.stowed && !asked;
        let tall = match (ui.session.manual, stowing) {
            (true, true) => f32::MAX,
            (true, false) => (ui.split.manual * tokens.scale).floor(),
            (false, _) => tokens.bar,
        };
        held = held.drawer(tall, tokens.sm);
        if stowing {
            held = held.stowed(!ui.session.manual);
        }
        held = match ui.session.alone {
            true => held.alone(),
            false => held,
        };
        if ui.rail.stowed && ui.rail.folded {
            held = held.over_rail();
        }
        held
    }

    /// The agent stretched left to the window's edge, under the rail's two folded headers.
    fn over_rail(mut self) -> Self {
        let grow = (self.agent.x - self.window.x).max(0.0);
        self.agent = Rect::new(
            self.window.x,
            self.agent.y,
            self.agent.w + grow,
            self.agent.h,
        );
        self
    }

    /// The sessions' window folded to its header, `tall` high; the feed, unless folded too,
    /// rising to a `gap` under it.
    fn sessions_folded(mut self, tall: f32, gap: f32, feed_folded: bool) -> Self {
        self.sessions.h = tall.min(self.sessions.h);
        if !feed_folded {
            let top = self.sessions.bottom() + gap;
            let foot = self.feed.bottom();
            self.feed = Rect::new(self.feed.x, top, self.feed.w, (foot - top).max(0.0));
        }
        self
    }

    /// The terminals' drawer under the strip, `tall` high, the strip ending a `gap` above it.
    pub fn drawer(mut self, tall: f32, gap: f32) -> Self {
        let strip = self.strip;
        let tall = tall.min(strip.h - self.header.h - gap).max(0.0);
        self.manual = Rect::new(strip.x, strip.bottom() - tall, strip.w, tall);
        let end = self.manual.y - gap;
        for rect in [
            &mut self.strip,
            &mut self.work,
            &mut self.workspace,
            &mut self.sidebar,
        ] {
            rect.h = (end - rect.y).max(0.0);
        }
        self
    }

    /// The strip folded to its header; the agent wide under it when `wide`.
    pub fn stowed(mut self, wide: bool) -> Self {
        if wide {
            let board = self.board;
            self.agent = Rect::new(board.x, self.agent.y, board.w, self.agent.h);
        }
        self.strip = Rect::new(self.strip.x, self.strip.y, self.strip.w, self.header.h);
        let none = Rect::new(self.window.w, self.window.h, 0.0, 0.0);
        (self.info, self.workspace, self.sidebar) = (none, none, none);
        self.work = none;
        self
    }

    /// The agent pane over everything right of the rail; the workspace's parts empty.
    pub fn alone(mut self) -> Self {
        let (rail, window) = (self.rail.w, self.window);
        self.agent = Rect::new(rail, self.agent.y, (window.w - rail).max(0.0), self.agent.h);
        let none = Rect::new(window.w, window.h, 0.0, 0.0);
        (self.header, self.workspace, self.sidebar) = (none, none, none);
        (self.manual, self.work) = (none, none);
        (self.strip, self.info) = (none, none);
        self
    }

    /// Where the agent's grid starts inside its pane, under the bar.
    pub fn agent_origin(&self, tokens: &Tokens) -> (f32, f32) {
        (self.agent.x + tokens.sm, self.agent.y + tokens.sm)
    }

    /// The columns and rows the agent pane holds at this cell size.
    pub fn agent_grid(&self, tokens: &Tokens, cell: CellSize) -> (u16, u16) {
        grid_in(self.agent, tokens, cell)
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
