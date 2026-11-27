//! One row per changed file: what it is, what it counts.

use groove_gfx::{Edges, Rect};
use groove_types::{FileDiff, FileStatus};

use super::reads_as;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{square, stand};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::changes;

/// How a file's row reads: whether it is the open one, and whether it carries a note.
#[derive(Debug, Clone, Copy)]
pub(super) struct Reading {
    pub open: bool,
    pub noted: bool,
    /// The file's errors and warnings, as its servers last found them.
    pub tally: Option<(u32, u32)>,
}

pub(super) fn entry(ctx: &mut Ctx, line: Rect, file: &FileDiff, indent: f32, reading: Reading) {
    let on_row = pointed(ctx, &file.path);
    if on_row {
        groove_ui_kit::widgets::glass_lit(ctx, line);
    }
    if reading.open {
        groove_ui_kit::widgets::glass_chosen(ctx, line);
    }
    let letter = file.status.letter().to_string();
    let at = line.pad(Edges::across(indent, 0.0));
    Label::new(&letter, ctx.styles.small(status_role(file.status))).draw(ctx, at);
    ctx.hit(line, Target::File(file.path.clone()));

    let sm = ctx.tokens.sm;
    let mut room = line.pad(Edges::across(super::text_at(ctx, indent), 0.0));
    counts(ctx, &mut room, file);
    if let Some(tally) = reading.tally {
        groove_ui_kit::widgets::faults(ctx, &mut room, tally);
    }
    let right = groove_ui_kit::widgets::Side::Right;
    if reading.noted {
        let size = ctx.tokens.small;
        stand(&mut room, sm, 0.0, right);
        let mark = square(stand(&mut room, size, 0.0, right), size);
        groove_ui_kit::widgets::icon(ctx, mark, Mark::Note, Role::Faint);
    }
    stand(&mut room, sm, 0.0, right);
    named(ctx, room, file);
}

/// The file's name; its directory is the row above it.
fn named(ctx: &mut Ctx, room: Rect, file: &FileDiff) {
    let (name, _) = reads_as(&file.path);
    Label::new(&name, ctx.styles.body(Role::Text)).draw(ctx, room);
}

/// The colour of a file's status letter.
pub(super) fn status_role(status: FileStatus) -> Role {
    match status {
        FileStatus::Added | FileStatus::Untracked => Role::Ok,
        FileStatus::Modified => Role::Warn,
        FileStatus::Deleted => Role::Bad,
        FileStatus::Renamed => Role::Accent,
        FileStatus::Unchanged => Role::Ghost,
    }
}

/// Whether the pointer is on this row.
fn pointed(ctx: &Ctx, path: &str) -> bool {
    matches!(ctx.hover(), Some(Target::File(at)) if at == path)
}

fn counts(ctx: &mut Ctx, room: &mut Rect, file: &FileDiff) {
    changes(ctx, room, (file.added, file.deleted), |ctx, role| {
        ctx.styles.small(role)
    });
}
