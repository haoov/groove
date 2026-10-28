mod scope;
mod tabs;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::views::session::resources::ResourcesUi;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::motion::turn;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::strip;
use groove_ui_kit::text::{Label, elide, row};
use groove_ui_kit::widgets::{Button, glass_pane, lead, picker};

/// The strip's first line: what the session is, then the tabs.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui) {
    let top = ctx.app.layout.header;
    let Some(open) = app.session.selected() else {
        let room = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new("Groove", ctx.styles.title(Role::Text)).draw(ctx, room);
        return;
    };
    let rest = tabs::draw(ctx, top, app, ui);
    let page = app
        .task
        .worked(&open.session)
        .and_then(|one| one.url.as_deref());
    if !ctx.app.layout.work.is_empty() {
        return titled(ctx, rest, open, page);
    }
    let wide = (titled_width(ctx, open, page)).min(rest.w);
    let line = strip(rest, rest.w - wide, wide);
    titled(ctx, line, open, page);
    glass_pane(ctx, strip(top, line.x - top.x, top.right() - line.x));
}

/// What the title needs: its mark, its words, the link to its page, and the padding around.
fn titled_width(ctx: &mut Ctx, open: &Open, page: Option<&str>) -> f32 {
    let (md, sm, icon) = (ctx.tokens.md, ctx.tokens.sm, ctx.tokens.icon);
    let words = ctx.measure(&open.session.title, &ctx.styles.session_title(Role::Text));
    let link = match page {
        Some(_) => icon + sm * 2.0 + sm,
        None => 0.0,
    };
    md * 2.0 + icon + sm + words + link
}

/// The line under the header: the pickers, the MR and the sidebar's fold, or a discard question.
pub fn info(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let line = groove_ui_kit::shape::band(ctx.app.layout.info, 0.0, ctx.tokens.row);
    if ui.losing() == Some(&crate::Losing::Everything) {
        let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
        return super::files::asking(ctx, room, "discard every change?");
    }
    let line = match ui.session.tab.has_sidebar() {
        true => super::fold(ctx, line, ui),
        false => line,
    };
    pickers(ctx, line, app, open);
}

/// The Kubernetes tab's line: the contexts and the namespaces it reads.
pub fn scope(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open, held: &ResourcesUi) {
    scope::draw(ctx, line, app, open, held);
}

/// The session's kind and its title, cut where the actions begin, then what opens its page.
fn titled(ctx: &mut Ctx, line: Rect, open: &Open, page: Option<&str>) {
    let (md, size) = (ctx.tokens.md, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(md, md));
    lead(
        ctx,
        &mut room,
        Mark::of_kind(&open.session.kind),
        Role::Faint,
    );
    let style = ctx.styles.session_title(Role::Text);
    let wide = match page {
        Some(_) => size + ctx.tokens.sm * 2.0 + ctx.tokens.sm,
        None => 0.0,
    };
    let title = elide(ctx, &open.session.title, &style, (room.w - wide).max(0.0));
    let fixed = |width: f32| Spec::default().width(width);
    let drawn = fixed(ctx.measure(&title, &style));
    let [_, _, after] = row_in(room, [drawn, fixed(ctx.tokens.sm), Spec::fill()]);
    row(ctx, room, 0.0, &title, style);
    if let Some(url) = page {
        let target = Target::TaskPage(url.to_string());
        let outward =
            Button::icon(Mark::Outward, 0, target, Role::Faint).hover(ctx.styles.glass_hover());
        outward.at(ctx, line, after.x);
    }
}

/// The pickers every tab follows, each label cut to the room the line has, the MR after them.
fn pickers(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let worktree = open.selected_worktree();
    let held = open.selected_repo();
    let (repo, branch) = named(open);
    let style = ctx.styles.body(Role::Text);
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let x = lead(ctx, &mut room, Mark::Git, Role::Faint).right() + ctx.tokens.sm;
    let delivery = worktree.map(|w| app.delivery.row(&w.id, open.status_of(&w.id)));
    let until = match delivery {
        Some(_) => refresh(ctx, line, app),
        None => line.right(),
    };
    let mr = delivery.as_ref().map_or(0.0, |one| room_for(ctx, one));
    let room = (until - ctx.tokens.md - x - around(ctx) - mr).max(0.0);
    let repo_room = ctx.measure(repo, &style).min(room / 2.0);
    let repo_text = elide(ctx, repo, &style, repo_room);
    let branch_text = elide(ctx, branch, &style, room - repo_room);

    let (band, hover) = (ctx.styles.band(), ctx.styles.glass_hover());
    let repo = picker(
        &repo_text,
        Target::Picker(Picks::Repo),
        role_of(held.is_some()),
        band,
        hover,
    )
    .flat();
    let box_ = repo.at(ctx, line, x);
    let x = box_.right() + ctx.tokens.sm;
    let role = role_of(worktree.is_some());
    let branch = picker(
        &branch_text,
        Target::Picker(Picks::Branch),
        role,
        band,
        hover,
    )
    .flat();
    let box_ = branch.at(ctx, line, x);
    if let Some(delivery) = delivery {
        delivered(ctx, line, box_.right() + ctx.tokens.sm, &delivery);
    }
}

/// What reads the MR again, turning while a read is out.
fn refresh(ctx: &mut Ctx, line: Rect, app: &AppState) -> f32 {
    let out = app
        .session
        .selected_worktree()
        .is_some_and(|worktree| app.delivery.poll.is_out(&worktree.id));
    let (turning, role) = match out {
        true => (turn(ctx.tick), Role::Working),
        false => (0, Role::Faint),
    };
    let hover = ctx.styles.glass_hover();
    let refresh = Button::icon(Mark::Busy, turning, Target::Refresh, role);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    refresh.lit(role).hover(hover).right(ctx, &mut room, 0.0).x
}

/// What the two pickers add around their labels, and the gaps after each.
fn around(ctx: &mut Ctx) -> f32 {
    let (band, hover) = (ctx.styles.band(), ctx.styles.glass_hover());
    let bare = picker("", Target::Picker(Picks::Repo), Role::Text, band, hover).width(ctx);
    (bare + ctx.tokens.sm) * 2.0
}

/// The repo and the branch the pickers name, each as it stands or as what it lacks.
fn named(open: &Open) -> (&str, &str) {
    let repo = open.selected_repo().map(|r| r.project.as_str());
    let branch = open.selected_worktree().map(|w| w.branch.as_str());
    (repo.unwrap_or("no repo"), branch.unwrap_or("no worktree"))
}

fn role_of(held: bool) -> Role {
    match held {
        true => Role::Text,
        false => Role::Ghost,
    }
}
