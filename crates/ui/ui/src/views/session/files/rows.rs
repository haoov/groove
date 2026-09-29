//! One row per changed file: what it is, what it offers, what it counts.

use groove_gfx::{Edges, Rect};
use groove_types::FileDiff;

use super::{Listing, acted, asking, reads_as};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;
use crate::{Losing, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{ruled, square};
use groove_ui_kit::text::{Label, elide, row};
use groove_ui_kit::widgets::changes;

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
    listed(
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
        return asking(ctx, line, "discard changes?");
    }
    let on_row = pointed(ctx, &file.path);
    if on_row {
        let hover = ctx.styles.hover();
        ctx.quad(line, hover);
    }
    if reading.open {
        ruled(ctx, line, ctx.styles.chosen());
    }
    let letter = file.status.letter().to_string();
    let at = line.pad(Edges::across(indent, 0.0));
    Label::new(&letter, ctx.styles.small(Role::Ghost)).draw(ctx, at);
    ctx.hit(line, Target::File(file.path.clone()));

    let sm = ctx.tokens.sm;
    let mut room = line.pad(Edges::across(indent + ctx.tokens.md, 0.0));
    match on_row {
        true => offer(ctx, &mut room, file),
        false => counts(ctx, &mut room, file),
    }
    if reading.noted {
        room.take_right(sm);
        let size = ctx.tokens.small;
        let mark = square(room.take_right(size), size);
        ctx.icon(mark, Mark::Note, 0, ctx.styles.color(Role::Faint));
    }
    room.take_right(sm);
    named(ctx, room, file);
}

/// The file's name, then what is left of its path behind it, dimmed.
fn named(ctx: &mut Ctx, mut room: Rect, file: &FileDiff) {
    let (name, behind) = reads_as(&file.path);
    let strong = ctx.styles.body(Role::Text);
    Label::new(&name, strong).left(ctx, &mut room, ctx.tokens.sm);
    if !behind.is_empty() && room.w > 0.0 {
        Label::new(&behind, ctx.styles.small(Role::Ghost)).draw(ctx, room);
    }
}

/// Whether the pointer is on this row, or on what the row is offering.
fn pointed(ctx: &Ctx, path: &str) -> bool {
    match ctx.hover() {
        Some(Target::File(at) | Target::Stage(at) | Target::Unstage(at)) => at == path,
        _ => false,
    }
}

/// What the row offers the pointer, or its counts when it has no change to stage.
fn offer(ctx: &mut Ctx, room: &mut Rect, file: &FileDiff) {
    let Some(staged) = file.staged else {
        return counts(ctx, room, file);
    };
    let (label, target) = match staged {
        true => ("unstage", Target::Unstage(file.path.clone())),
        false => ("stage", Target::Stage(file.path.clone())),
    };
    acted(ctx, room, label, target)
}

fn counts(ctx: &mut Ctx, room: &mut Rect, file: &FileDiff) {
    changes(ctx, room, (file.added, file.deleted), |ctx, role| {
        ctx.styles.small(role)
    });
}
