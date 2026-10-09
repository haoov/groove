mod scope;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::SessionKind;

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::motion::turn;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::{Label, elide, row};
use groove_ui_kit::widgets::{Button, Text, lead, picker};

/// The workspace's first lines: what the session is, what it points at, what Resources reads.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui) {
    let rect = ctx.app.layout.header;
    groove_ui_kit::shape::ground(ctx, rect, Ground::Work);
    hairline(ctx, rect, ctx.styles.line());

    let tall = |height: f32| Spec::default().height(height);
    let row = tall(ctx.tokens.row);
    let [top, under, third, _] = column_in(rect, [tall(ctx.tokens.header), row, row, Spec::fill()]);
    let Some(open) = app.session.selected() else {
        let room = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new("Groove", ctx.styles.title(Role::Text)).draw(ctx, room);
        return;
    };
    let until = actions(ctx, top, app, open);
    let page = app
        .task
        .worked(&open.session)
        .and_then(|one| one.url.as_deref());
    titled(ctx, top.until(until), open, page);
    pickers(ctx, under, app, open);
    scope::draw(ctx, third, app, open, &ui.session.resources);
}

/// The session's own actions. Returns where they start, which the title stops at.
fn actions(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) -> f32 {
    if matches!(open.session.kind, SessionKind::Explorer) {
        return line.right();
    }
    let more = menu_caret(ctx, line, open);
    if !finishable(app, open) {
        return more;
    }
    let target = Target::Finish(open.session.id.clone());
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let finish = Button::new("finish", target, Role::Ok, band)
        .text(Text::Label)
        .hover(hover);
    let mut room = line.until(more).pad(Edges::across(0.0, ctx.tokens.sm));
    finish.right(ctx, &mut room, 0.0).x
}

/// What opens the rest of them, at the line's right end.
fn menu_caret(ctx: &mut Ctx, line: Rect, open: &Open) -> f32 {
    let target = Target::TaskActions(open.session.id.clone());
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let more = Button::icon(Mark::Down, 0, target, Role::Muted)
        .ground(band)
        .hover(hover);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    more.right(ctx, &mut room, 0.0).x
}

/// Whether the session works a task with no worktree still carrying an open MR.
fn finishable(app: &AppState, open: &Open) -> bool {
    matches!(open.session.kind, SessionKind::Task { .. })
        && app.delivery.all_landed(&open.worktrees)
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
    let style = ctx.styles.title(Role::Text);
    let wide = match page {
        Some(_) => size + ctx.tokens.sm * 2.0 + ctx.tokens.sm,
        None => 0.0,
    };
    let title = elide(ctx, &open.session.title, &style, (room.w - wide).max(0.0));
    let end = room.x + ctx.measure(&title, &style);
    row(ctx, room, 0.0, &title, style);
    if let Some(url) = page {
        let target = Target::TaskPage(url.to_string());
        let outward = Button::icon(Mark::Outward, 0, target, Role::Faint).hover(ctx.styles.hover());
        outward.at(ctx, line, end + ctx.tokens.sm);
    }
}

/// The pickers every tab follows, each label cut to the room the line has, the MR after them.
fn pickers(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let worktree = open.selected_worktree();
    let held = worktree.and_then(|w| open.repos.iter().find(|r| r.id == w.repo));
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

    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let repo = picker(
        &repo_text,
        Target::Picker(Picks::Repo),
        role_of(held.is_some()),
        band,
        hover,
    );
    let box_ = repo.at(ctx, line, x);
    let x = box_.right() + ctx.tokens.sm;
    let role = role_of(worktree.is_some());
    let branch = picker(
        &branch_text,
        Target::Picker(Picks::Branch),
        role,
        band,
        hover,
    );
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
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let refresh = Button::icon(Mark::Busy, turning, Target::Refresh, role);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    refresh
        .lit(role)
        .ground(band)
        .hover(hover)
        .right(ctx, &mut room, 0.0)
        .x
}

/// What the two pickers add around their labels, and the gaps after each.
fn around(ctx: &mut Ctx) -> f32 {
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let bare = picker("", Target::Picker(Picks::Repo), Role::Text, band, hover).width(ctx);
    (bare + ctx.tokens.sm) * 2.0
}

/// The repo and the branch the pickers name, each as it stands or as what it lacks.
fn named(open: &Open) -> (&str, &str) {
    let worktree = open.selected_worktree();
    let repo = worktree
        .and_then(|w| open.repos.iter().find(|r| r.id == w.repo))
        .map(|r| r.project.as_str());
    let branch = worktree.map(|w| w.branch.as_str());
    (repo.unwrap_or("no repo"), branch.unwrap_or("no worktree"))
}

fn role_of(held: bool) -> Role {
    match held {
        true => Role::Text,
        false => Role::Ghost,
    }
}
