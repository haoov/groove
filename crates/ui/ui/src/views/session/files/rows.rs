//! One row per changed file: what it is, what it offers, what it counts.

use groove_gfx::Rect;
use groove_types::FileDiff;

use super::{Listing, acted, asking, reads_as};
use crate::base::ctx::Ctx;
use crate::base::hit::{Scroller, Target};
use crate::base::style::Role;
use crate::shape::ruled;
use crate::text::{elide, row};
use crate::widgets::scrolled;
use crate::{Losing, Ui};

enum Item<'a> {
    Dir(&'a str, Role),
    File(&'a FileDiff, f32),
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &groove_controllers::AppState,
    listing: &Listing<'_>,
    open: Option<&String>,
    ui: &Ui,
) {
    let height = ctx.tokens.row;
    let items = items(ctx, listing);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    scrolled(
        ctx,
        body,
        at,
        &items,
        |_| height,
        |ctx, line, item| match item {
            Item::Dir(text, role) => path(ctx, line, text, *role),
            Item::File(file, indent) => {
                let reading = Reading {
                    open: open == Some(&file.path),
                    noted: super::noted(app, &file.path),
                };
                entry(ctx, line, file, *indent, reading, ui)
            }
        },
    );
}

/// The root, then each directory over its files.
fn items<'a>(ctx: &Ctx, listing: &'a Listing<'_>) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    if !listing.root.is_empty() {
        items.push(Item::Dir(&listing.root, Role::Ghost));
    }
    for group in &listing.groups {
        let indent = match group.dir.is_empty() {
            true => ctx.tokens.md,
            false => {
                items.push(Item::Dir(&group.dir, Role::Faint));
                ctx.tokens.md + ctx.tokens.md
            }
        };
        items.extend(group.files.iter().map(|file| Item::File(file, indent)));
    }
    items
}

/// A row naming a directory, elided to the room it has.
fn path(ctx: &mut Ctx, line: Rect, text: &str, role: Role) {
    let style = ctx.styles.small(role);
    let room = line.w - ctx.tokens.md * 2.0;
    let text = elide(ctx, text, &style, room);
    row(ctx, line, ctx.tokens.md, &text, style);
}

/// How a file's row reads: whether it is the open one, and whether it carries a note.
#[derive(Debug, Clone, Copy)]
pub(super) struct Reading {
    pub open: bool,
    pub noted: bool,
}

pub(super) fn entry(
    ctx: &mut Ctx,
    line: Rect,
    file: &FileDiff,
    indent: f32,
    reading: Reading,
    ui: &Ui,
) {
    if ui.losing() == Some(&Losing::File(file.path.clone())) {
        return asking(ctx, line, "discard changes?", ui);
    }
    let on_row = pointed(ui, &file.path);
    if on_row {
        let hover = ctx.styles.hover();
        ctx.quad(line, hover);
    }
    if reading.open {
        let here = ctx.styles.here();
        ruled(ctx, line, here);
    }
    let letter = ctx.styles.small(Role::Ghost);
    let mark = file.status.letter().to_string();
    row(ctx, line, indent, &mark, letter);

    ctx.hit(line, Target::File(file.path.clone()));
    let at = match on_row {
        true => offer(ctx, line, file, ui),
        false => counts(ctx, line, file),
    };
    let at = match reading.noted {
        true => marked(ctx, line, at),
        false => at,
    };
    let start = indent + ctx.tokens.md;
    let room = (at - line.x - start - ctx.tokens.sm).max(0.0);
    named(
        ctx,
        Rect::new(line.x, line.y, at - line.x, line.h),
        file,
        start,
        room,
    );
}

/// The file's name, then what is left of its path behind it, dimmed.
fn named(ctx: &mut Ctx, line: Rect, file: &FileDiff, start: f32, room: f32) {
    let (name, behind) = reads_as(&file.path);
    let strong = ctx.styles.body(Role::Text);
    let quiet = ctx.styles.small(Role::Ghost);
    let text = elide(ctx, &name, &strong, room);
    let width = ctx.measure(&text, &strong);
    row(ctx, line, start, &text, strong);
    let left = room - width - ctx.tokens.sm;
    if behind.is_empty() || left <= 0.0 {
        return;
    }
    let rest = elide(ctx, &behind, &quiet, left);
    let at = line.x + start + width + ctx.tokens.sm;
    row(ctx, Rect::new(at, line.y, left, line.h), 0.0, &rest, quiet);
}

/// The mark a file carrying a note takes, left of its counts. Returns where it starts.
fn marked(ctx: &mut Ctx, line: Rect, at: f32) -> f32 {
    let size = ctx.tokens.small;
    let x = at - size - ctx.tokens.sm;
    let box_ = Rect::new(x, line.y + (line.h - size) / 2.0, size, size);
    ctx.icon(
        box_,
        crate::base::mark::Mark::Note,
        0,
        ctx.styles.color(Role::Faint),
    );
    x
}

/// Whether the pointer is on this row, or on what the row is offering.
fn pointed(ui: &Ui, path: &str) -> bool {
    match &ui.hover {
        Some(Target::File(at) | Target::Stage(at) | Target::Unstage(at)) => at == path,
        _ => false,
    }
}

/// What the row offers the pointer, or its counts when it has no change to stage.
fn offer(ctx: &mut Ctx, line: Rect, file: &FileDiff, ui: &Ui) -> f32 {
    let Some(staged) = file.staged else {
        return counts(ctx, line, file);
    };
    let (label, target) = match staged {
        true => ("unstage", Target::Unstage(file.path.clone())),
        false => ("stage", Target::Stage(file.path.clone())),
    };
    acted(ctx, line, label, target, ui)
}

/// Returns where the counts start.
fn counts(ctx: &mut Ctx, line: Rect, file: &FileDiff) -> f32 {
    let mut at = line.right() - ctx.tokens.md;
    for (count, role) in [(file.deleted, Role::Bad), (file.added, Role::Ok)] {
        if count == 0 {
            continue;
        }
        let style = ctx.styles.small(role);
        let sign = match role {
            Role::Ok => '+',
            _ => '-',
        };
        let text = format!("{sign}{count}");
        let width = ctx.measure(&text, &style);
        at -= width;
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at -= ctx.tokens.sm;
    }
    at
}
