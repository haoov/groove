mod children;
mod clusters;
mod log;
mod task;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{LogEntry, Repo, Session, SessionKind, Task, Worktree};

use crate::Ui;
use crate::components::worktree_row;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_from};
use groove_ui_kit::shape::{after_mark, top_rule};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Heading, Row, flat_row, list, panel_head, sunk_pane};

/// One band of the overview, top to bottom.
enum Part<'a> {
    /// The card a section stands on, as tall as its head and its items; drawn first, under them.
    Card(f32),
    /// The section's head band: its name, and how many it holds when it counts.
    Head(&'a str, Option<usize>),
    /// The task's properties, and whether it is marked as maintenance.
    Properties(&'a Task, bool),
    Child(&'a Session),
    Entry(&'a LogEntry),
    NoLog,
    NoRepos,
    Repo(&'a Repo),
    Worktree(&'a Worktree),
    /// A context the session holds, in the hue Settings gave it, with its namespaces.
    Cluster(&'a str, Option<groove_types::Hue>, Vec<Option<&'a str>>),
    /// The body as Markdown, as tall as it lays out.
    Body(&'a str, f32),
    Gap,
}

/// The overview tab, scrolled: each section a heading over a card of its items.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    // As far off the strip's bottom edge as the first card stands under the info line's rule.
    let area = area.pad(Edges {
        bottom: ctx.tokens.sm,
        ..Edges::all(0.0)
    });
    let parts = parts(ctx, app, open, area.w);
    let heights: Vec<f32> = parts.iter().map(|part| height(ctx, part)).collect();
    let specs: Vec<Spec> = heights.iter().map(|h| Spec::default().height(*h)).collect();
    let tall: f32 = heights.iter().sum();
    let rects = column_from(area, ui.session.overview, &specs);
    ctx.hit(area, Target::Overview);
    ctx.clipped(area, |ctx| {
        let mut ruled = false;
        for (part, rect) in parts.iter().zip(rects) {
            let row = matches!(
                part,
                Part::Repo(_) | Part::Worktree(_) | Part::Cluster(..) | Part::Child(_) | Part::Entry(_) | Part::NoLog
            );
            if row && ruled {
                top_rule(ctx, inset(ctx, rect), ctx.styles.line());
            }
            ruled = row;
            draw_part(ctx, app, open, (area, rect), part);
        }
    });
    ctx.app
        .hits
        .scrolls(Scroller::Overview, (tall + ctx.tokens.md - area.h).max(0.0));
}

/// Every band the session's overview holds, in order.
fn parts<'a>(ctx: &mut Ctx, app: &'a AppState, open: &'a Open, wide: f32) -> Vec<Part<'a>> {
    let mut out = Vec::new();
    let worked = app.task.worked(&open.session);
    if let Some(one) = worked {
        section(
            ctx,
            &mut out,
            ("Properties", None),
            vec![Part::Properties(one, open.state.maintenance)],
        );
    }
    let mut repos = Vec::new();
    if open.repos.is_empty() {
        repos.push(Part::NoRepos);
    }
    for repo in &open.repos {
        repos.push(Part::Repo(repo));
        let held = open.worktrees.iter().filter(|w| w.repo == repo.id);
        repos.extend(held.map(Part::Worktree));
    }
    let held = Some(open.repos.len());
    section(ctx, &mut out, ("Repos and worktrees", held), repos);
    if open.holds_clusters() {
        let parts = clusters::parts(app, open);
        let count = Some(parts.len());
        section(ctx, &mut out, ("Clusters", count), parts);
    }
    maintained(ctx, &mut out, open);
    logged(ctx, &mut out, open);
    let (title, text) = match &open.session.kind {
        SessionKind::Review { .. } => ("Description", described(app, open)),
        _ => ("Body", worked.and_then(|one| app.task.body(&one.short_id))),
    };
    if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
        let tall = task::body_height(ctx, text, wide);
        section(ctx, &mut out, (title, None), vec![Part::Body(text, tall)]);
    }
    out
}

/// A maintenance task's children; nothing on another session.
fn maintained<'a>(ctx: &Ctx, out: &mut Vec<Part<'a>>, open: &'a Open) {
    if !open.state.maintenance && open.children.is_empty() {
        return;
    }
    let children: Vec<Part> = open.children.iter().map(Part::Child).collect();
    let count = Some(children.len());
    section(ctx, out, ("Children", count), children);
}

/// The log of a task or an explorer, the newest entry first.
fn logged<'a>(ctx: &Ctx, out: &mut Vec<Part<'a>>, open: &'a Open) {
    if !matches!(open.session.kind, SessionKind::Task { .. } | SessionKind::Explorer) {
        return;
    }
    let mut entries: Vec<Part> = open.log.iter().map(Part::Entry).collect();
    let count = Some(entries.len());
    if entries.is_empty() {
        entries.push(Part::NoLog);
    }
    section(ctx, out, ("Log", count), entries);
}

/// A card holding its head band, then `items`.
fn section<'a>(
    ctx: &Ctx,
    out: &mut Vec<Part<'a>>,
    (title, count): (&'a str, Option<usize>),
    items: Vec<Part<'a>>,
) {
    let head = Part::Head(title, count);
    let tall = height(ctx, &head) + items.iter().map(|one| height(ctx, one)).sum::<f32>();
    out.extend([Part::Card(tall), head]);
    out.extend(items);
    out.push(Part::Gap);
}

fn height(ctx: &Ctx, part: &Part) -> f32 {
    let row = ctx.tokens.row;
    match part {
        Part::Card(_) => 0.0,
        Part::Gap => ctx.tokens.md,
        Part::Body(_, tall) => tall + ctx.tokens.md * 2.0,
        Part::Properties(..) => row * 2.0 + ctx.tokens.sm * 2.0,
        Part::Entry(one) => log::height(ctx, one),
        Part::Cluster(..) => row + ctx.tokens.sm,
        Part::Head(..) | Part::NoRepos | Part::Repo(_) | Part::Worktree(_) | Part::Child(_) | Part::NoLog => {
            row + ctx.tokens.xs
        }
    }
}

fn inset(ctx: &Ctx, rect: Rect) -> Rect {
    rect.pad(Edges::across(ctx.tokens.md, ctx.tokens.md))
}

fn draw_part(ctx: &mut Ctx, app: &AppState, open: &Open, (area, rect): (Rect, Rect), part: &Part) {
    let pad = ctx.tokens.md;
    let line = groove_ui_kit::shape::band(rect, (rect.h - ctx.tokens.row) / 2.0, ctx.tokens.row);
    match part {
        Part::Card(tall) => {
            let held = groove_ui_kit::shape::band(inset(ctx, rect), 0.0, *tall);
            sunk_pane(ctx, held);
        }
        Part::Head(title, count) => {
            let band = inset(ctx, rect);
            panel_head(ctx, band, false);
            let named = Heading::new(title);
            let named = match count {
                Some(count) => named.count(*count),
                None => named,
            };
            named.draw(ctx, band);
        }
        Part::Properties(one, maintenance) => {
            task::properties(ctx, inset(ctx, rect), one, *maintenance)
        }
        Part::Child(one) => children::child(ctx, app, inset(ctx, rect), one),
        Part::Entry(one) => log::entry(ctx, inset(ctx, rect), one),
        Part::NoLog => {
            let line = inset(ctx, line).pad(Edges::across(pad, pad));
            let style = ctx.styles.body(Role::Faint);
            Label::new("Nothing logged yet.", style).draw(ctx, line);
        }
        Part::NoRepos => {
            let style = ctx.styles.body(Role::Faint);
            let line = inset(ctx, line).pad(Edges::across(pad, pad));
            Label::new("No repos. Add one from the palette.", style).draw(ctx, line);
        }
        Part::Repo(repo) => repo_row(ctx, inset(ctx, line), repo),
        Part::Worktree(worktree) => {
            let target = Target::Worktree(worktree.id.clone());
            flat_row(ctx, inset(ctx, rect), target, false);
            let delivery = app.delivery.row(&worktree.id, open.status_of(&worktree.id));
            worktree_row::draw(ctx, inset(ctx, line), worktree, Some(&delivery));
        }
        Part::Cluster(context, hue, held) => {
            clusters::draw(ctx, inset(ctx, line), (context, *hue), held)
        }
        Part::Body(text, _) => task::body(ctx, area, inset(ctx, rect), text),
        Part::Gap => {}
    }
}

/// A repo's own row: its name, then its slug right after it.
fn repo_row(ctx: &mut Ctx, line: Rect, repo: &Repo) {
    let pad = ctx.tokens.md;
    let (name, slug) = (ctx.styles.label(Role::Text), ctx.styles.small(Role::Faint));
    let at_slug = after_mark(ctx, pad) + ctx.measure(&repo.project, &name) + ctx.tokens.sm;
    let head =
        Row::new(pad, &repo.project, name)
            .mark(Mark::Repo)
            .aside(at_slug, repo.id.as_str(), slug);
    list(ctx, line, &[head], None);
}

/// The description of the MR the session's worktrees hold, once its forge has answered.
fn described<'a>(app: &'a AppState, open: &Open) -> Option<&'a str> {
    open.worktrees.iter().find_map(|worktree| {
        let read = app.delivery.held(&worktree.id)?.read.as_ref()?;
        Some(read.details.description.as_str())
    })
}
