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
use groove_ui_kit::layout::{Spec, column_of};
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Row, list};

/// One band of the overview, top to bottom.
enum Part<'a> {
    /// The rule over a section that follows another.
    Rule,
    Heading(&'a str),
    Properties(&'a Task, Option<TimeSummary>),
    NoRepos,
    Repo(&'a Repo),
    Worktree(&'a Worktree),
    /// A context the session holds, in the hue Settings gave it, then one of its namespaces.
    Cluster(&'a str, Option<groove_types::Hue>),
    Namespace(Option<&'a str>),
    Gap,
    /// The body as Markdown; it stands as tall as it draws.
    Body(&'a str),
}

/// The overview tab, scrolled: the properties, the repos with their worktrees, the body.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let parts = parts(app, open);
    let heights: Vec<f32> = parts.iter().map(|part| height(ctx, part)).collect();
    let specs: Vec<Spec> = heights.iter().map(|h| Spec::default().height(*h)).collect();
    let top = area.y - ui.session.overview;
    let tall: f32 = heights.iter().sum();
    let rects = column_of(
        Rect {
            y: top,
            h: tall,
            ..area
        },
        &specs,
    );
    let mut prose = 0.0;
    ctx.clipped(area, |ctx| {
        for (part, rect) in parts.iter().zip(rects) {
            prose += draw_part(ctx, app, open, (area, rect), part);
        }
    });
    let height = tall + prose + ctx.tokens.md;
    ctx.app
        .hits
        .scrolls(Scroller::Overview, (height - area.h).max(0.0));
}

/// Every band the session's overview holds, in order.
fn parts<'a>(app: &'a AppState, open: &'a Open) -> Vec<Part<'a>> {
    let mut parts = Vec::new();
    let worked = app.task.worked(&open.session);
    if let Some(one) = worked {
        let time = app.task.measured(&one.external_id);
        parts.extend([
            Part::Heading("Properties"),
            Part::Properties(one, time),
            Part::Gap,
        ]);
        parts.push(Part::Rule);
    }
    parts.push(Part::Heading("Repos and worktrees"));
    if open.repos.is_empty() {
        parts.push(Part::NoRepos);
    }
    for repo in &open.repos {
        parts.push(Part::Repo(repo));
        let held = open.worktrees.iter().filter(|w| w.repo == repo.id);
        parts.extend(held.map(Part::Worktree));
        parts.push(Part::Gap);
    }
    if !open.clusters.is_empty() {
        parts.extend([Part::Rule, Part::Heading("Clusters")]);
        parts.extend(clusters::parts(app, open));
    }
    let (title, text) = match &open.session.kind {
        SessionKind::Review { .. } => ("Description", described(app, open)),
        _ => ("Body", worked.and_then(|one| app.task.body(&one.short_id))),
    };
    if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
        parts.extend([Part::Rule, Part::Heading(title), Part::Body(text)]);
    }
    parts
}

fn height(ctx: &Ctx, part: &Part) -> f32 {
    let row = ctx.tokens.row;
    match part {
        Part::Rule | Part::Gap => ctx.tokens.sm,
        Part::Properties(..) => row * task::PROPERTIES as f32,
        Part::Body(_) => 0.0,
        Part::Heading(_) | Part::NoRepos | Part::Repo(_) | Part::Worktree(_) => row,
        Part::Cluster(..) | Part::Namespace(_) => row,
    }
}

/// Draws one band in its rect; returns how tall the body stood, which no box held.
fn draw_part(
    ctx: &mut Ctx,
    app: &AppState,
    open: &Open,
    (area, rect): (Rect, Rect),
    part: &Part,
) -> f32 {
    let pad = ctx.tokens.md;
    match part {
        Part::Rule => groove_ui_kit::shape::top_rule(
            ctx,
            rect.pad(Edges::across(pad, pad)),
            ctx.styles.line(),
        ),
        Part::Heading(title) => groove_ui_kit::widgets::Heading::new(title).draw(ctx, rect),
        Part::Properties(one, time) => task::properties(ctx, rect, one, *time),
        Part::NoRepos => {
            let style = ctx.styles.body(Role::Faint);
            let line = rect.pad(Edges::across(pad, pad));
            Label::new("No repos. Add one from the palette.", style).draw(ctx, line);
        }
        Part::Repo(repo) => repo_row(ctx, rect, repo),
        Part::Worktree(worktree) => {
            ctx.hit(rect, Target::Worktree(worktree.id.clone()));
            let delivery = app.delivery.row(&worktree.id, open.status_of(&worktree.id));
            worktree_row::draw(ctx, rect, worktree, Some(&delivery));
        }
        Part::Cluster(context, hue) => clusters::context(ctx, rect, context, *hue),
        Part::Namespace(namespace) => clusters::namespace(ctx, rect, *namespace),
        Part::Gap => {}
        Part::Body(text) => return task::body(ctx, area, rect, text),
    }
    0.0
}

/// A repo's own row: its name, then its slug at the aside column.
fn repo_row(ctx: &mut Ctx, line: Rect, repo: &Repo) {
    let pad = ctx.tokens.md;
    let (name, slug) = (ctx.styles.label(Role::Text), ctx.styles.small(Role::Faint));
    let named = after_mark(ctx, pad) + ctx.measure(&repo.project, &name) + ctx.tokens.md;
    let at_slug = named.max(ctx.tokens.aside_mid);
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

/// Names down the left and what each holds at the aside column, a row each.
fn table(ctx: &mut Ctx, rect: Rect, held: &[(&str, &str)]) {
    let (label, value) = (ctx.styles.body(Role::Faint), ctx.styles.body(Role::Text));
    let (md, at) = (ctx.tokens.md, ctx.tokens.aside_near + ctx.tokens.md);
    let rows: Vec<Row<'_, _>> = held
        .iter()
        .map(|(name, held)| Row::new(md, name, label).aside(at, held, value))
        .collect();
    list(ctx, rect, &rows, None);
}
