use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::box_in;
use crate::text::{Label, row};

/// The room `counts` needs, for a caller placing it against a right edge.
pub fn counts_room<A: App>(ctx: &mut Ctx<'_, A>, items: &[(Mark, u32, Role)]) -> f32 {
    let mut wide = 0.0;
    for (_, value, role) in items.iter().filter(|(_, value, _)| *value > 0) {
        let style = ctx.styles.small(*role);
        wide += style.size + ctx.tokens.xs;
        wide += ctx.measure(&value.to_string(), &style) + ctx.tokens.md;
    }
    wide
}

/// Icon and number pairs from `x`, zeros left out; returns the x after the last.
pub fn counts<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    x: f32,
    items: &[(Mark, u32, Role)],
) -> f32 {
    let gap = ctx.tokens.xs;
    let mut at = x;
    for (mark, value, role) in items.iter().filter(|(_, value, _)| *value > 0) {
        let style = ctx.styles.small(*role);
        let color = ctx.styles.color(*role);
        let box_ = box_in(line, at, style.size);
        ctx.icon(box_, *mark, 0, color);
        at += style.size + gap;
        let text = value.to_string();
        let width = ctx.measure(&text, &style);
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at += width + ctx.tokens.md;
    }
    at
}

/// What a file lost and gained, from the right of `room`.
pub fn changes<A: App>(
    ctx: &mut Ctx<'_, A>,
    room: &mut Rect,
    (added, deleted): (u32, u32),
    styled: impl Fn(&Ctx<'_, A>, Role) -> TextStyle,
) {
    room.take_right(ctx.tokens.md);
    for (count, role, sign) in [(deleted, Role::Bad, '-'), (added, Role::Ok, '+')] {
        if count > 0 {
            let text = format!("{sign}{count}");
            Label::new(&text, styled(ctx, role)).right(ctx, room, ctx.tokens.sm);
        }
    }
}

fn tallied((errors, warnings): (u32, u32)) -> [(Mark, u32, Role); 2] {
    [
        (Mark::Errors, errors, Role::Bad),
        (Mark::Warnings, warnings, Role::Warn),
    ]
}

/// The room `faults` needs.
pub fn faults_room<A: App>(ctx: &mut Ctx<'_, A>, tally: (u32, u32)) -> f32 {
    counts_room(ctx, &tallied(tally))
}

/// How many errors and warnings a file has, from the right of `room`.
pub fn faults<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, tally: (u32, u32)) {
    let items = tallied(tally);
    let wide = counts_room(ctx, &items);
    if wide > 0.0 {
        let at = room.take_right(wide);
        counts(ctx, at, at.x, &items);
    }
}
