//! What a search of the worktree found: each file over its own matches.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Found;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{elide, row, scrolled};

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
            Item::Match(at, one) => hit(ctx, line, one, *at, ui),
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
    let target = Target::FoundIn(path.to_string());
    ctx.quad(line, ctx.styles.raised());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let style = ctx.styles.small(Role::Text);
    let label = format!("{count}");
    let width = ctx.measure(&label, &style);
    let at = line.right() - ctx.tokens.md - width;
    row(
        ctx,
        Rect::new(at, line.y, width, line.h),
        0.0,
        &label,
        style,
    );
    let size = ctx.tokens.icon;
    let caret = Rect::new(
        line.x + ctx.tokens.xs,
        line.y + (line.h - size) / 2.0,
        size,
        size,
    );
    let turn = match ui.session.shut.contains(path) {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(caret, Mark::Down, turn, ctx.styles.color(Role::Faint));
    let start = caret.right() - line.x + ctx.tokens.xs;
    let room = (at - line.x - start - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, path, &style, room);
    row(ctx, line, start, &text, style);
}

/// One line a search matched: where it sits, and what it says.
fn hit(ctx: &mut Ctx, line: Rect, one: &Found, at: usize, ui: &Ui) {
    let target = Target::Found(at);
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let numbers = ctx.styles.code(Role::Ghost);
    let number = format!("{}", one.line + 1);
    let width = ctx.measure(&number, &numbers);
    let start = ctx.tokens.md + ctx.tokens.sm;
    row(
        ctx,
        Rect::new(line.x + start, line.y, width, line.h),
        0.0,
        &number,
        numbers,
    );
    let style = ctx.styles.code(Role::Text);
    let at = start + width + ctx.tokens.sm;
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let text = elide(ctx, one.text.trim_start(), &style, room);
    row(ctx, line, at, &text, style);
}
