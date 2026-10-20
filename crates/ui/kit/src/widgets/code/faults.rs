//! What a language server found on a row: a wavy line under each range, its number in the worst colour.

use groove_gfx::{Color, Rect};

use super::Code;
use super::paint::upto;
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::widgets::wavy;

pub(super) fn faulted<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
    let style = ctx.styles.source(Role::Text);
    for (from, to, severity) in code.faults {
        let start = upto(ctx, code.text, *from, &style);
        let end = upto(ctx, code.text, *to, &style);
        let color = ctx.styles.fault(*severity);
        wavy(ctx, rect, (start, end), color);
    }
}

/// The colour of the worst of them, for the row's number.
pub(super) fn worst<A: App>(ctx: &Ctx<'_, A>, code: &Code<'_, A::Target>) -> Option<Color> {
    let severity = code.faults.iter().map(|(_, _, severity)| *severity).min()?;
    Some(ctx.styles.fault(severity))
}
