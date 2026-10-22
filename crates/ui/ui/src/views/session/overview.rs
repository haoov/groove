mod clusters;
mod task;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{Repo, SessionKind, Task, TimeSummary, Worktree};

use crate::Ui;
use crate::components::worktree_row;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_from};
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Row, list};

/// One band of the overview, top to bottom.
enum Part<'a> {
    /// A section's name, above its items.
    Heading(&'a str),
    /// The hairline between two sections.
    Rule,
    Properties(&'a Task, Option<TimeSummary>),
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
    let parts = parts(ctx, app, open, area.w);
    let heights: Vec<f32> = parts.iter().map(|part| height(ctx, part)).collect();
    let specs: Vec<Spec> = heights.iter().map(|h| Spec::default().height(*h)).collect();
    let tall: f32 = heights.iter().sum();
    let rects = column_from(area, ui.session.overview, &specs);
    ctx.hit(area, Target::Overview);
    ctx.clipped(area, |ctx| {
        for (part, rect) in parts.iter().zip(rects) {
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
        let time = app.task.measured(&one.external_id);
        section(&mut out, "Properties", vec![Part::Properties(one, time)]);
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
    section(&mut out, "Repos and worktrees", repos);
    if open.holds_clusters() {
        section(&mut out, "Clusters", clusters::parts(app, open));
    }
    let (title, text) = match &open.session.kind {
        SessionKind::Review { .. } => ("Description", described(app, open)),
        _ => ("Body", worked.and_then(|one| app.task.body(&one.short_id))),
    };
    if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
        let tall = task::body_height(ctx, text, wide);
        section(&mut out, title, vec![Part::Body(text, tall)]);
    }
    out
}

/// A hairline after the section before, then the heading and its `items`.
fn section<'a>(out: &mut Vec<Part<'a>>, title: &'a str, items: Vec<Part<'a>>) {
    if !out.is_empty() {
        out.push(Part::Rule);
    }
    out.push(Part::Heading(title));
    out.extend(items);
    out.push(Part::Gap);
}

fn height(ctx: &Ctx, part: &Part) -> f32 {
    let row = ctx.tokens.row;
    match part {
        Part::Rule => ctx.tokens.hairline,
        Part::Gap => ctx.tokens.sm,
        Part::Body(_, tall) => tall + ctx.tokens.md * 2.0,
        Part::Properties(..) | Part::Cluster(..) => row + ctx.tokens.sm,
        Part::Heading(_) | Part::NoRepos | Part::Repo(_) | Part::Worktree(_) => row + ctx.tokens.xs,
    }
}

fn inset(ctx: &Ctx, rect: Rect) -> Rect {
    rect.pad(Edges::across(ctx.tokens.xs, ctx.tokens.xs))
}

fn draw_part(ctx: &mut Ctx, app: &AppState, open: &Open, (area, rect): (Rect, Rect), part: &Part) {
    let pad = ctx.tokens.md;
    let line = groove_ui_kit::shape::band(rect, (rect.h - ctx.tokens.row) / 2.0, ctx.tokens.row);
    match part {
        Part::Heading(title) => groove_ui_kit::widgets::Heading::new(title).draw(ctx, rect),
        Part::Rule => groove_ui_kit::shape::top_rule(
            ctx,
            inset(ctx, rect).pad(Edges::across(pad, pad)),
            ctx.styles.line(),
        ),
        Part::Properties(one, time) => task::properties(ctx, inset(ctx, line), one, *time),
        Part::NoRepos => {
            let style = ctx.styles.body(Role::Faint);
            let line = inset(ctx, line).pad(Edges::across(pad, pad));
            Label::new("No repos. Add one from the palette.", style).draw(ctx, line);
        }
        Part::Repo(repo) => repo_row(ctx, inset(ctx, line), repo),
        Part::Worktree(worktree) => {
            let target = Target::Worktree(worktree.id.clone());
            groove_ui_kit::widgets::inset_row(ctx, inset(ctx, rect), target, (false, None));
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
