//! The Files tab's strip: one tab per open file, a dot while it owes the disk, a cross to close it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Losing, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hairline, hoverable, square};
use groove_ui_kit::text::Label;

pub fn draw(ctx: &mut Ctx, strip: Rect, app: &AppState, ui: &Ui) {
    ctx.quad(strip, ctx.styles.band());
    hairline(ctx, strip, ctx.styles.line());
    let open = app
        .workspace
        .buffers()
        .map(|one| one.all())
        .unwrap_or_default();
    if let Some(path) = losing(ui, open) {
        let question = format!("close {} and lose its edits?", name(path));
        return crate::views::session::files::asking(ctx, strip, &question);
    }
    if open.is_empty() {
        let room = strip.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let said = "no file open: pick one in the explorer";
        Label::new(said, ctx.styles.small(Role::Faint)).draw(ctx, room);
        return;
    }
    let active = app.workspace.active().map(|one| one.path.as_str());
    let mut room = strip;
    for one in open {
        tab(ctx, &mut room, one, active == Some(one.path.as_str()));
    }
}

/// One file's tab from the left of `room`.
fn tab(ctx: &mut Ctx, room: &mut Rect, open: &Opened, active: bool) {
    let (sm, md, size) = (ctx.tokens.sm, ctx.tokens.md, ctx.tokens.small);
    let role = match active {
        true => Role::Text,
        false => Role::Muted,
    };
    let label = Label::new(name(&open.path), ctx.styles.small(role));
    let width = label.width(ctx) + md * 2.0 + sm + size;
    let box_ = room.take_left(width);
    let rule = room.take_left(ctx.tokens.hairline);
    ctx.quad(rule, ctx.styles.line());
    if active {
        ctx.quad(box_, ctx.styles.ground());
    }
    hoverable(ctx, box_, Target::OpenTab(open.path.clone()));
    let mut inside = box_.pad(Edges::across(md, md));
    let cross = square(inside.take_right(size), size);
    inside.take_right(sm);
    label.draw(ctx, inside);
    let close = Target::CloseTab(open.path.clone());
    let mark = match open.new.dirty() && !ctx.hovered(&close) {
        true => Mark::Modified,
        false => Mark::Close,
    };
    ctx.icon(cross, mark, 0, ctx.styles.color(Role::Faint));
    ctx.hit(cross, close);
}

/// The open file whose closing waits on an answer.
fn losing<'a>(ui: &Ui, open: &'a [Opened]) -> Option<&'a str> {
    let Some(Losing::Tab(path)) = ui.losing() else {
        return None;
    };
    open.iter()
        .map(|one| one.path.as_str())
        .find(|one| one == path)
}

fn name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
