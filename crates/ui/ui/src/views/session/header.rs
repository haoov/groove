use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::SessionKind;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{
    after_mark, box_in, button, elide, hairline, icon, leading, picker, row, slot,
};

/// The workspace's two first lines: what the session is, then what it points at.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.layout.header;
    ctx.quad(rect, ctx.styles.ground());
    hairline(ctx, rect, ctx.styles.line());

    let top = Rect::new(rect.x, rect.y, rect.w, ctx.tokens.header);
    let under = Rect::new(rect.x, top.bottom(), rect.w, ctx.tokens.row);
    let Some(open) = app.session.selected() else {
        let style = ctx.styles.title(Role::Text);
        return row(ctx, top, ctx.tokens.md, "Groove", style);
    };
    let until = actions(ctx, top, open, ui);
    titled(ctx, top, open, until);
    pickers(ctx, under, open, ui);
}

/// The task's own actions. Returns where they start, which the title stops at.
fn actions(ctx: &mut Ctx, line: Rect, open: &Open, ui: &Ui) -> f32 {
    if !matches!(open.session.kind, SessionKind::Task { .. }) {
        return line.right();
    }
    let more = menu_caret(ctx, line, open, ui);
    if !finishable(open) {
        return more;
    }
    let target = Target::Finish(open.session.id.clone());
    let ground = match ui.hover.as_ref() == Some(&target) {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let room = Rect::new(line.x, line.y, more - line.x, line.h);
    let box_ = button(
        ctx,
        room,
        "finish",
        ctx.styles.label(Role::Ok),
        Some(ground),
    );
    ctx.hit(box_, target);
    box_.x
}

/// What opens the rest of them, at the line's right end.
fn menu_caret(ctx: &mut Ctx, line: Rect, open: &Open, ui: &Ui) -> f32 {
    let target = Target::TaskActions(open.session.id.clone());
    let ground = match ui.hover.as_ref() == Some(&target) {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let size = ctx.tokens.small;
    let box_ = slot(ctx, line, size, Some(ground));
    let caret = box_in(box_, box_.x + (box_.w - size) / 2.0, size);
    ctx.icon(caret, Mark::Down, 0, ctx.styles.color(Role::Muted));
    ctx.hit(box_, target);
    box_.x
}

/// Whether the session works a task with no worktree still carrying an open MR.
fn finishable(open: &Open) -> bool {
    matches!(open.session.kind, SessionKind::Task { .. })
        && open.worktrees.iter().all(|worktree| {
            open.delivery_of(&worktree.id)
                .is_none_or(|one| !one.is_open())
        })
}

/// The session's kind and its title, cut where the actions begin.
fn titled(ctx: &mut Ctx, line: Rect, open: &Open, until: f32) {
    let pad = ctx.tokens.md;
    let box_ = leading(ctx, line, line.x + pad);
    icon(ctx, box_, Mark::of_kind(&open.session.kind), Role::Faint);
    let style = ctx.styles.title(Role::Text);
    let at = after_mark(ctx, pad);
    let room = (until - line.x - at - pad).max(0.0);
    let text = elide(ctx, &open.session.title, &style, room);
    row(ctx, line, at, &text, style);
}

/// The pickers every tab follows, each label cut to the room the line has.
fn pickers(ctx: &mut Ctx, line: Rect, open: &Open, ui: &Ui) {
    let worktree = open.selected_worktree();
    let held = worktree.and_then(|w| open.repos.iter().find(|r| r.id == w.repo));
    let (repo, branch) = named(open);
    let style = ctx.styles.body(Role::Muted);
    let x = line.x + ctx.tokens.md;
    let room = (line.right() - ctx.tokens.md - x - around(ctx)).max(0.0);
    let repo_room = ctx.measure(repo, &style).min(room / 2.0);
    let repo_text = elide(ctx, repo, &style, repo_room);
    let branch_text = elide(ctx, branch, &style, room - repo_room);

    let role = role_of(held.is_some());
    let box_ = picker(ctx, line, x, &repo_text, role, lit(ui, Picks::Repo));
    ctx.hit(box_, Target::Picker(Picks::Repo));
    let x = box_.right() + ctx.tokens.sm;
    let role = role_of(worktree.is_some());
    let box_ = picker(ctx, line, x, &branch_text, role, lit(ui, Picks::Branch));
    ctx.hit(box_, Target::Picker(Picks::Branch));
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

fn lit(ui: &Ui, which: Picks) -> bool {
    ui.hover.as_ref() == Some(&Target::Picker(which))
}

fn role_of(held: bool) -> Role {
    match held {
        true => Role::Muted,
        false => Role::Ghost,
    }
}
