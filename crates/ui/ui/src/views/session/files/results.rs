//! What a search of the worktree found: each file over its own matches.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Found;
use groove_gfx::Rect;

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::{Scroller, Target};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::hoverable;
use crate::shape::square;
use crate::text::Label;
use crate::widgets::scrolled;
use groove_gfx::Edges;

enum Item<'a> {
    File(&'a str, usize),
    Match(usize, &'a Found),
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let height = ctx.tokens.row;
    let items = items(&app.workspace.found, ui);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    scrolled(
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

/// The file a run of matches belongs to, how many, and a caret that hides them.
fn file_found(ctx: &mut Ctx, line: Rect, path: &str, count: usize, ui: &Ui) {
    ctx.quad(line, ctx.styles.raised());
    hoverable(ctx, line, Target::FoundIn(path.to_string()));
    let (xs, sm, size) = (ctx.tokens.xs, ctx.tokens.sm, ctx.tokens.icon);
    let style = ctx.styles.small(Role::Text);
    let mut room = line.pad(Edges::across(xs, ctx.tokens.md));
    Label::new(&count.to_string(), style).right(ctx, &mut room, sm);
    let caret = square(room.take_left(size), size);
    room.take_left(xs);
    let turn = match ui.session.shut.contains(path) {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(caret, Mark::Down, turn, ctx.styles.color(Role::Faint));
    Label::new(path, style).draw(ctx, room);
}

/// One line a search matched: where it sits, and what it says.
fn hit(ctx: &mut Ctx, line: Rect, one: &Found, at: usize) {
    hoverable(ctx, line, Target::Found(at));
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md + sm, md));
    let number = (one.line + 1).to_string();
    Label::new(&number, ctx.styles.code(Role::Ghost)).left(ctx, &mut room, sm);
    Label::new(one.text.trim_start(), ctx.styles.code(Role::Text)).draw(ctx, room);
}
