mod task;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::SessionKind;

use crate::Ui;
use crate::components::worktree_row;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Row, list};

/// The overview tab, scrolled: the properties, the repos with their worktrees, the body.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let top = area.y - ui.session.overview;
    let mut column = Rect {
        y: top,
        h: f32::INFINITY,
        ..area
    };
    ctx.clipped(area, |ctx| {
        let placed = properties(ctx, app, open, &mut column);
        section(ctx, &mut column, "Repos and worktrees", placed);
        repos(ctx, app, open, &mut column);
        body(ctx, app, open, area, &mut column);
    });
    let height = column.y - top + ctx.tokens.md;
    ctx.app
        .hits
        .scrolls(Scroller::Overview, (height - area.h).max(0.0));
}

/// The task's six properties, for a session that works one; whether it drew them.
fn properties(ctx: &mut Ctx, app: &AppState, open: &Open, column: &mut Rect) -> bool {
    let Some(one) = app.task.worked(&open.session) else {
        return false;
    };
    section(ctx, column, "Properties", false);
    let time = app.task.measured(&one.external_id);
    task::properties(ctx, column, one, time);
    column.take_top(ctx.tokens.sm);
    true
}

/// The task's body, or the MR's description in a review, under everything the session holds.
fn body(ctx: &mut Ctx, app: &AppState, open: &Open, area: Rect, column: &mut Rect) {
    let (title, text) = match &open.session.kind {
        SessionKind::Review { .. } => ("Description", described(app, open)),
        _ => {
            let worked = app.task.worked(&open.session);
            ("Body", worked.and_then(|one| app.task.body(&one.short_id)))
        }
    };
    if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
        section(ctx, column, title, true);
        task::body(ctx, area, column, text);
    }
}

/// The description of the MR the session's worktrees hold, once its forge has answered.
fn described<'a>(app: &'a AppState, open: &Open) -> Option<&'a str> {
    open.worktrees.iter().find_map(|worktree| {
        let read = app.delivery.held(&worktree.id)?.read.as_ref()?;
        Some(read.details.description.as_str())
    })
}

/// Names down the left and what each holds at the aside column, a row each.
fn table(ctx: &mut Ctx, column: &mut Rect, held: &[(&str, &str)]) -> Rect {
    let (label, value) = (ctx.styles.body(Role::Faint), ctx.styles.body(Role::Text));
    let (md, at) = (ctx.tokens.md, ctx.tokens.aside_near + ctx.tokens.md);
    let rows: Vec<Row<'_, _>> = held
        .iter()
        .map(|(name, held)| Row::new(md, name, label).aside(at, held, value))
        .collect();
    let taken = column.take_top(ctx.tokens.row * rows.len() as f32);
    list(ctx, taken, &rows, None);
    taken
}

/// A section's name, under a rule when one stands above it.
fn section(ctx: &mut Ctx, column: &mut Rect, title: &str, under: bool) {
    let pad = ctx.tokens.md;
    if under {
        let rule = Rect::new(
            column.x + pad,
            column.y,
            column.w - pad * 2.0,
            ctx.tokens.hairline,
        );
        groove_ui_kit::shape::top_rule(ctx, rule, ctx.styles.line());
        column.take_top(ctx.tokens.sm);
    }
    let line = column.take_top(ctx.tokens.row).pad(Edges::across(pad, pad));
    Label::new(&title.to_uppercase(), ctx.styles.heading(Role::Faint)).draw(ctx, line);
}

/// One block per repo: the repo, then its worktrees.
fn repos(ctx: &mut Ctx, app: &AppState, open: &Open, column: &mut Rect) {
    let pad = ctx.tokens.md;
    if open.repos.is_empty() {
        let line = column.take_top(ctx.tokens.row).pad(Edges::across(pad, pad));
        let style = ctx.styles.body(Role::Faint);
        Label::new("No repos. Add one from the palette.", style).draw(ctx, line);
        return;
    }
    let (name, slug) = (ctx.styles.label(Role::Text), ctx.styles.small(Role::Faint));
    for repo in &open.repos {
        let named = after_mark(ctx, pad) + ctx.measure(&repo.project, &name) + ctx.tokens.md;
        let at_slug = named.max(ctx.tokens.aside_mid);
        let head = Row::new(pad, &repo.project, name).mark(Mark::Repo).aside(
            at_slug,
            repo.id.as_str(),
            slug,
        );
        list(ctx, column.take_top(ctx.tokens.row), &[head], None);
        for worktree in open.worktrees.iter().filter(|w| w.repo == repo.id) {
            let line = column.take_top(ctx.tokens.row);
            ctx.hit(line, Target::Worktree(worktree.id.clone()));
            let delivery = app.delivery.row(&worktree.id, open.status_of(&worktree.id));
            worktree_row::draw(ctx, line, worktree, Some(&delivery));
        }
        column.take_top(ctx.tokens.sm);
    }
}
