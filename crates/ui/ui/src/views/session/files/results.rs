//! What a search of the worktree found: each file over its own matches.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Found;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::folder;

use groove_gfx::Edges;

enum Item<'a> {
    File(&'a str, usize),
    Match(usize, &'a Found),
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let height = ctx.tokens.row;
    let items = items(&app.workspace.found, ui);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    listed(
        ctx,
        body,
        at,
        &items,
        |_| height,
        |ctx, line, item| match item {
            Item::File(path, count) => file_found(ctx, line, path, *count, ui),
            Item::Match(at, one) => hit(ctx, line, one, *at),
        },
    );
}

/// Each file once, over its matches unless it is shut.
fn items<'a>(found: &'a [Found], ui: &Ui) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut over: Option<&str> = None;
    for (at, one) in found.iter().enumerate() {
        if over != Some(one.path.as_str()) {
            let count = found.iter().filter(|it| it.path == one.path).count();
            items.push(Item::File(&one.path, count));
            over = Some(one.path.as_str());
        }
        if !ui.session.shut.contains(&one.path) {
            items.push(Item::Match(at, one));
        }
    }
    items
}

/// The file a run of matches belongs to, as the changed list draws a directory, and its count.
fn file_found(ctx: &mut Ctx, line: Rect, path: &str, count: usize, ui: &Ui) {
    hoverable(ctx, line, Target::FoundIn(path.to_string()));
    let md = ctx.tokens.md;
    let mut room = line.pad(Edges::across(md, md));
    let counted = ctx.styles.small(Role::Faint);
    Label::new(&count.to_string(), counted).right(ctx, &mut room, ctx.tokens.sm);
    folder(ctx, &mut room, !ui.session.shut.contains(path), Role::Faint);
    Label::new(path, ctx.styles.body(Role::Faint)).draw(ctx, room);
}

/// One line a search matched: its number, then what it says, in the column a name stands in.
fn hit(ctx: &mut Ctx, line: Rect, one: &Found, at: usize) {
    hoverable(ctx, line, Target::Found(at));
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md + ctx.tokens.icon + ctx.tokens.xs, md));
    let number = (one.line + 1).to_string();
    Label::new(&number, ctx.styles.prose_code(Role::Ghost)).left(ctx, &mut room, sm);
    Label::new(one.text.trim_start(), ctx.styles.prose_code(Role::Text)).draw(ctx, room);
}
