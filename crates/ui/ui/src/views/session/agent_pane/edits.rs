//! The files the agent wrote back to back, as one block: each a link to its hunk in the diff.

use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::chat::{Edit, Entry};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::from_top;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{changes, glass_hoverable};

use super::parts::{boxed, cap, cap_right, head};
use crate::ctx::Ctx;
use crate::hit::Target;

fn written(edits: &[Entry]) -> impl Iterator<Item = &Edit> {
    edits.iter().filter_map(|one| match one {
        Entry::Edit(edit) => Some(edit),
        _ => None,
    })
}

pub(super) fn height(ctx: &Ctx, edits: &[Entry]) -> f32 {
    (written(edits).count() + 1) as f32 * ctx.tokens.row
}

/// `EDITED · 3 FILES` and what they gained and lost, then a row for each file.
pub(super) fn draw(ctx: &mut Ctx, open: &Open, whole: Rect, edits: &[Entry]) {
    boxed(ctx, whole);
    let mut room = whole;
    let mut top = head(ctx, from_top(&mut room, ctx.tokens.row, 0.0));
    let count = written(edits).count();
    let title = match count {
        1 => "edited · 1 file".to_string(),
        many => format!("edited · {many} files"),
    };
    let sum = |(a, r): (u32, u32), one: &Edit| {
        let (plus, minus) = one.patch.map_or((0, 0), |p| (p.added, p.removed));
        (a + plus, r + minus)
    };
    let total = written(edits).fold((0, 0), sum);
    changes(ctx, &mut top, total, |ctx, role| ctx.styles.log(role));
    cap(ctx, &mut top, &title, Role::Faint);
    for edit in written(edits) {
        let line = from_top(&mut room, ctx.tokens.row, 0.0);
        file(ctx, open, line, edit);
    }
}

/// One file: its path in the accent, a link to its first hunk, and its `+/−`.
fn file(ctx: &mut Ctx, open: &Open, line: Rect, edit: &Edit) {
    let at = edit.patch.map_or(1, |one| one.line);
    let target = Target::Edited {
        path: edit.path.clone(),
        line: at,
    };
    glass_hoverable(ctx, line, target);
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    if let Some(patch) = edit.patch {
        let counts = (patch.added, patch.removed);
        changes(ctx, &mut room, counts, |ctx, role| ctx.styles.log(role));
    }
    if edit.failed {
        cap_right(ctx, &mut room, "failed", Role::Bad);
    } else if edit.created {
        cap_right(ctx, &mut room, "new", Role::Ok);
    }
    let shown = super::within(open, &edit.path).map_or_else(|| tail(&edit.path), |(_, rel)| rel);
    Label::new(&shown, ctx.styles.code(Role::Accent))
        .cut_start()
        .draw(ctx, room);
}

/// A path outside the session's worktrees, by its last two parts.
fn tail(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(2).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}
