//! The Overview tab in the modes layout: the task or the MR on a card, then each repo apart
//! with its worktrees in rows, each card folding on its head, scrolled on the ground.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{MrDetails, Repo, SessionKind, Task, Worktree};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, band, from_top, mid_rule};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{cap, cap_right, flat_row, fold, panel_head};

use super::task;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

pub mod aside;
mod repo;

/// One block of the tab, top to bottom; a card's last field says it stands open.
enum Block<'a> {
    /// The task, its body as tall as it lays out.
    Task(&'a Task, Option<&'a str>, f32, bool),
    /// The MR under review, its description as tall as it lays out.
    Review(&'a MrDetails, String, f32, bool),
    /// The rule over the repos, with how many.
    Repos(usize),
    Repo(&'a Repo, Vec<&'a Worktree>, bool),
}

const TASK: &str = "task";
const REVIEW: &str = "review";

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let column = area.pad(Edges::xy(ctx.tokens.xl, ctx.tokens.sm));
    let blocks = blocks(ctx, (app, ui), open, column.w);
    let gap = ctx.tokens.lg;
    let tall: Vec<f32> = blocks.iter().map(|one| height(ctx, one)).collect();
    let whole: f32 = tall.iter().map(|one| one + gap).sum();
    ctx.hit(area, Target::Overview);
    let mut y = -ui.session.overview;
    ctx.clipped(area, |ctx| {
        for (one, h) in blocks.iter().zip(&tall) {
            if y + h > 0.0 && y < area.h {
                drawn(ctx, app, open, (area, band(column, y, *h)), one);
            }
            y += h + gap;
        }
    });
    let far = (whole - column.h).max(0.0);
    ctx.app.hits.scrolls(Scroller::Overview, far);
}

/// The session's blocks: its task or its MR, then its repos.
fn blocks<'a>(
    ctx: &mut Ctx,
    (app, ui): (&'a AppState, &Ui),
    open: &'a Open,
    wide: f32,
) -> Vec<Block<'a>> {
    let shown = |key: &str| !ui.session.cards.contains(key);
    let mut out = Vec::new();
    if let Some(one) = app.task.worked(&open.session) {
        let body = app
            .task
            .body(&one.short_id)
            .filter(|one| !one.trim().is_empty());
        let tall = body.map_or(0.0, |text| task::body_height(ctx, text, inner(ctx, wide)));
        out.push(Block::Task(one, body, tall, shown(TASK)));
    }
    if matches!(open.session.kind, SessionKind::Review { .. })
        && let Some((details, repo)) = reviewed(app, open)
    {
        let tall = task::body_height(ctx, &details.description, inner(ctx, wide));
        out.push(Block::Review(details, repo, tall, shown(REVIEW)));
    }
    out.push(Block::Repos(open.repos.len()));
    for one in &open.repos {
        let held = open.worktrees.iter().filter(|w| w.repo == one.id).collect();
        out.push(Block::Repo(one, held, shown(one.id.as_str())));
    }
    out
}

/// The width `body_height` takes for a body drawn across a card `wide`.
fn inner(ctx: &Ctx, wide: f32) -> f32 {
    wide + ctx.tokens.md * 2.0
}

/// The MR the review's worktrees hold, and the repo it stands in, once its forge has answered.
fn reviewed<'a>(app: &'a AppState, open: &Open) -> Option<(&'a MrDetails, String)> {
    open.worktrees.iter().find_map(|worktree| {
        let read = app.delivery.held(&worktree.id)?.read.as_ref()?;
        let repo = open.repo_of(worktree).map(|one| one.project.clone());
        Some((&read.details, repo.unwrap_or_default()))
    })
}

fn height(ctx: &Ctx, one: &Block) -> f32 {
    let (bar, row, md) = (ctx.tokens.bar, ctx.tokens.row, ctx.tokens.md);
    match one {
        Block::Task(_, None, _, _) | Block::Task(.., false) | Block::Review(.., false) => {
            bar + row + md * 2.0
        }
        Block::Task(_, Some(_), body, true) => bar + row + body + md * 2.0 + md,
        Block::Review(_, _, body, true) => bar + row * 2.0 + body + md * 2.0 + md,
        Block::Repos(_) => row,
        Block::Repo(.., false) => bar,
        Block::Repo(_, held, true) => bar + held.len().max(1) as f32 * row,
    }
}

fn drawn(ctx: &mut Ctx, app: &AppState, open: &Open, (area, rect): (Rect, Rect), one: &Block) {
    match one {
        Block::Task(task, body, _, shown) => {
            tasked(ctx, (area, rect), task, body.filter(|_| *shown), *shown)
        }
        Block::Review(details, repo, _, shown) => {
            reviewing(ctx, (area, rect), (details, repo), *shown)
        }
        Block::Repos(count) => {
            let mut line = rect;
            cap(ctx, &mut line, &format!("repos · {count}"), Role::Faint);
            mid_rule(ctx, line, ctx.styles.panel_edge());
        }
        Block::Repo(one, held, shown) => repo::draw(ctx, app, open, rect, (one, held, *shown)),
    }
}

/// The task on a card: its source, its title, its body while open.
fn tasked(ctx: &mut Ctx, (area, rect): (Rect, Rect), task: &Task, body: Option<&str>, shown: bool) {
    let mut room = carded(ctx, rect);
    let mut top = headed(ctx, &mut room, (TASK, shown));
    let source = format!("task · {}", task.short_id);
    cap(ctx, &mut top, &source, Role::Faint);
    titled(ctx, &mut room, &task.title);
    if let Some(text) = body {
        task::body(ctx, area, room, text);
    }
}

/// The MR under review on a card; its author and description only while open.
fn reviewing(
    ctx: &mut Ctx,
    (area, rect): (Rect, Rect),
    (details, repo): (&MrDetails, &str),
    shown: bool,
) {
    let mut room = carded(ctx, rect);
    let mut top = headed(ctx, &mut room, (REVIEW, shown));
    cap_right(ctx, &mut top, details.state.as_str(), Role::Accent);
    cap(ctx, &mut top, &format!("review · {repo}"), Role::Faint);
    titled(ctx, &mut room, &details.title);
    if !shown {
        return;
    }
    let by = from_top(&mut room, ctx.tokens.row, 0.0);
    let by = by.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let said = format!(
        "by {} · {} into {}",
        details.author, details.source_branch, details.target_branch
    );
    Label::new(&said, ctx.styles.small(Role::Faint)).draw(ctx, by);
    task::body(ctx, area, room, &details.description);
}

/// The title a gap under the head, taken from the top of `room`.
fn titled(ctx: &mut Ctx, room: &mut Rect, title: &str) {
    let line = from_top(room, ctx.tokens.row + ctx.tokens.md, 0.0);
    let line = line.pad(Edges {
        top: ctx.tokens.md,
        ..Edges::across(ctx.tokens.md, ctx.tokens.md)
    });
    Label::new(title, ctx.styles.session_title(Role::Text)).draw(ctx, line);
}

/// A block's card on the ground, a hairline around it; returns its room.
pub(super) fn carded(ctx: &mut Ctx, rect: Rect) -> Rect {
    let panel = Panel::default().ground(ctx.styles.ground());
    panel.border(ctx.styles.panel_edge()).draw(ctx, rect);
    rect
}

/// The card's head band, folding the card under `key`; returns the room inside it.
pub(super) fn headed(ctx: &mut Ctx, room: &mut Rect, (key, shown): (&str, bool)) -> Rect {
    let top = from_top(room, ctx.tokens.bar, 0.0);
    panel_head(ctx, top, !shown);
    flat_row(ctx, top, Target::CardFold(key.to_string()), false);
    let mut inner = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    fold(ctx, &mut inner, shown, Role::Faint);
    inner
}
