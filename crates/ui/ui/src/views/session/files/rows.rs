//! One row per changed file: what it is, what it offers, what it counts.

use groove_gfx::Rect;
use groove_types::FileDiff;

use super::{Listing, acted, asking, reads_as};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::style::Role;
use crate::widget::{elide, row, ruled};
use crate::{Losing, Ui};

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    listing: &Listing<'_>,
    open: Option<&String>,
    ui: &Ui,
) {
    let height = ctx.tokens.row;
    let extent = (height * lines(listing) as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = ui.session.files.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        if !listing.root.is_empty() {
            path(
                ctx,
                Rect::new(body.x, y, body.w, height),
                &listing.root,
                Role::Ghost,
            );
            y += height;
        }
        for group in &listing.groups {
            if !group.dir.is_empty() {
                path(
                    ctx,
                    Rect::new(body.x, y, body.w, height),
                    &group.dir,
                    Role::Faint,
                );
                y += height;
            }
            for file in &group.files {
                let line = Rect::new(body.x, y, body.w, height);
                let indent = match group.dir.is_empty() {
                    true => ctx.tokens.md,
                    false => ctx.tokens.md + ctx.tokens.md,
                };
                entry(ctx, line, file, indent, open == Some(&file.path), ui);
                y += height;
            }
        }
    });
}

/// How many rows the whole listing stands: its root, its groups, its files.
fn lines(listing: &Listing<'_>) -> usize {
    let groups: usize = listing
        .groups
        .iter()
        .map(|g| g.files.len() + usize::from(!g.dir.is_empty()))
        .sum();
    usize::from(!listing.root.is_empty()) + groups
}

/// A row naming a directory, elided to the room it has.
fn path(ctx: &mut Ctx, line: Rect, text: &str, role: Role) {
    let style = ctx.styles.small(role);
    let room = line.w - ctx.tokens.md * 2.0;
    let text = elide(ctx, text, &style, room);
    row(ctx, line, ctx.tokens.md, &text, style);
}

pub(super) fn entry(ctx: &mut Ctx, line: Rect, file: &FileDiff, indent: f32, open: bool, ui: &Ui) {
    if ui.discarding == Some(Losing::File(file.path.clone())) {
        return asking(ctx, line, "discard changes?", ui);
    }
    let on_row = pointed(ui, &file.path);
    if on_row {
        let hover = ctx.styles.hover();
        ctx.quad(line, hover);
    }
    if open {
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
