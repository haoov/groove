use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::SessionKind;

use crate::components::{delivered, room_for};
use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::motion::turn;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hairline, square};
use groove_ui_kit::text::{Label, elide};
use groove_ui_kit::widgets::{button, icon, mark_button, picker};

/// The workspace's two first lines: what the session is, then what it points at.
pub fn draw(ctx: &mut Ctx, app: &AppState) {
    let rect = ctx.app.layout.header;
    ctx.quad(rect, ctx.styles.ground());
    hairline(ctx, rect, ctx.styles.line());

    let mut body = rect;
    let top = body.take_top(ctx.tokens.header);
    let under = body.take_top(ctx.tokens.row);
    let Some(open) = app.session.selected() else {
        let room = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new("Groove", ctx.styles.title(Role::Text)).draw(ctx, room);
        return;
    };
    let until = actions(ctx, top, app, open);
    titled(ctx, top.until(until), open);
    pickers(ctx, under, app, open);
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
    let ground = match ctx.hovered(&target) {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let box_ = button(
        ctx,
        line.until(more),
        "finish",
        ctx.styles.label(Role::Ok),
        Some(ground),
    );
    ctx.hit(box_, target);
    box_.x
}

/// What opens the rest of them, at the line's right end.
fn menu_caret(ctx: &mut Ctx, line: Rect, open: &Open) -> f32 {
    let target = Target::TaskActions(open.session.id.clone());
    let grounds = (ctx.styles.band(), ctx.styles.hover());
    mark_button(ctx, line, target, (Mark::Down, 0, Role::Muted), grounds).x
}

/// Whether the session works a task with no worktree still carrying an open MR.
fn finishable(app: &AppState, open: &Open) -> bool {
    matches!(open.session.kind, SessionKind::Task { .. })
        && open
            .worktrees
            .iter()
            .all(|worktree| !app.delivery.is_open(&worktree.id))
}

/// The session's kind and its title, cut where the actions begin.
fn titled(ctx: &mut Ctx, line: Rect, open: &Open) {
    let (md, size) = (ctx.tokens.md, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(md, md));
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, mark, Mark::of_kind(&open.session.kind), Role::Faint);
    Label::new(&open.session.title, ctx.styles.title(Role::Text)).draw(ctx, room);
}

/// The pickers every tab follows, each label cut to the room the line has.
fn pickers(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) {
    let worktree = open.selected_worktree();
    let held = worktree.and_then(|w| open.repos.iter().find(|r| r.id == w.repo));
    let (repo, branch) = named(open);
    let style = ctx.styles.body(Role::Muted);
    let x = line.x + ctx.tokens.md;
    let until = forge(ctx, line, app, open);
    let room = (until - ctx.tokens.md - x - around(ctx)).max(0.0);
    let repo_room = ctx.measure(repo, &style).min(room / 2.0);
    let repo_text = elide(ctx, repo, &style, repo_room);
    let branch_text = elide(ctx, branch, &style, room - repo_room);

    let role = role_of(held.is_some());
    let box_ = picker(ctx, line, x, &repo_text, role, lit(ctx, Picks::Repo));
    ctx.hit(box_, Target::Picker(Picks::Repo));
    let x = box_.right() + ctx.tokens.sm;
    let role = role_of(worktree.is_some());
    let box_ = picker(ctx, line, x, &branch_text, role, lit(ctx, Picks::Branch));
    ctx.hit(box_, Target::Picker(Picks::Branch));
}

/// What the selected worktree's forge says, at the right end; the pickers stop there.
fn forge(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open) -> f32 {
    let Some(worktree) = open.selected_worktree() else {
        return line.right();
    };
    let until = refresh(ctx, line, app);
    let delivery = app.delivery.row(&worktree.id, open.status_of(&worktree.id));
    let wide = room_for(ctx, &delivery);
    if wide <= 0.0 {
        return until;
    }
    let x = until - ctx.tokens.sm - wide;
    delivered(ctx, line, x, &delivery);
    x
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
    let grounds = (ctx.styles.band(), ctx.styles.hover());
    mark_button(
        ctx,
        line,
        Target::Refresh,
        (Mark::Busy, turning, role),
        grounds,
    )
    .x
}

/// What the two buttons add around their labels: a caret and the padding each side.
fn around(ctx: &mut Ctx) -> f32 {
    let style = ctx.styles.body(Role::Muted);
    let box_ = ctx.tokens.sm * 2.0 + ctx.tokens.xs + style.size;
    box_ * 2.0 + ctx.tokens.sm
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

fn lit(ctx: &Ctx, which: Picks) -> bool {
    ctx.hovered(&Target::Picker(which))
}

fn role_of(held: bool) -> Role {
    match held {
        true => Role::Muted,
        false => Role::Ghost,
    }
}
