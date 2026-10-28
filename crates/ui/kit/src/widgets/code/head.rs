//! A file's head across the code surface: its fold, its counts, its name and its read box.

use groove_gfx::{Edges, Rect};

use super::{Code, head_mark};
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::text::Label;

/// A file's head, whole, drawn from whichever of its rows `line` is.
pub(super) fn head<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, code: &Code<'_, A::Target>) {
    let whole = super::head_of(line, code.part);
    let role = match code.read {
        true => Role::Faint,
        false => Role::Text,
    };
    let ground = whole.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.xs / 2.0));
    ctx.rounded(ground, ctx.styles.glass_hover(), ctx.tokens.sm);
    let (xs, size) = (ctx.tokens.xs, ctx.tokens.icon);
    let mut room = ground.pad(Edges::across(xs, ctx.tokens.sm));
    crate::widgets::caret(ctx, &mut room, (!code.folded, size), Role::Faint);
    room.take_right(ctx.tokens.md + size);
    if let Some(counts) = code.counts {
        crate::widgets::changes(ctx, &mut room, counts, |ctx, role| ctx.styles.code(role));
    }
    if let Some(tally) = code.tally {
        crate::widgets::faults(ctx, &mut room, tally);
    }
    let (folder, name) = match code.text.rsplit_once('/') {
        Some((folder, name)) => (folder, name),
        None => ("", code.text),
    };
    let named = Label::new(name, ctx.styles.file_head(role)).draw(ctx, room);
    room.take_left(named.w + ctx.tokens.sm);
    Label::new(folder, ctx.styles.body(Role::Faint))
        .cut_start()
        .draw(ctx, room);
    box_(ctx, head_mark(ctx, whole), code.read);
}

/// The box that says whether a file is read, ticked once it is.
fn box_<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, read: bool) {
    let square = rect.pad(Edges::all(ctx.tokens.xs / 2.0));
    let role = match read {
        true => Role::Text,
        false => Role::Faint,
    };
    ctx.border(square, ctx.styles.color(role));
    if read {
        ctx.icon(rect, Mark::Read, 0, ctx.styles.color(role));
    }
}
