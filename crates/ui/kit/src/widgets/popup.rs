//! A panel of lines under a point: what the language servers say of one spot.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::base::tokens::POPUP_LINES;
use crate::text::row;
use crate::widgets::{Corner, panel_at};

/// One part of what the panel says: its text, its colour, and whether it is code.
pub struct Said<'a> {
    pub text: &'a str,
    pub role: Role,
    pub code: bool,
}

/// The parts under `at`, each wrapped to the panel, `POPUP_LINES` at most. Returns the panel.
pub fn popup<A: App>(ctx: &mut Ctx<'_, A>, at: (f32, f32), parts: &[Said<'_>]) -> Rect {
    let (pad, line) = (ctx.tokens.sm, ctx.tokens.line);
    let width = ctx.tokens.popup.min(ctx.window.w - ctx.tokens.xl);
    let cols = (((width - pad * 2.0) / ctx.advance.max(1.0)) as usize).max(1);
    let lines: Vec<(String, Role, bool)> = parts
        .iter()
        .flat_map(|part| {
            let pieces = wrapped(part.text, cols);
            pieces.into_iter().map(|text| (text, part.role, part.code))
        })
        .take(POPUP_LINES)
        .collect();
    let height = lines.len() as f32 * line + pad * 2.0;
    let border = ctx.styles.line();
    let rect = panel_at(ctx, at, Corner::TopLeft, (width, height), border);
    let mut y = rect.y + pad;
    for (text, role, code) in &lines {
        let style = match code {
            true => ctx.styles.code(*role),
            false => ctx.styles.body(*role),
        };
        let at = Rect::new(rect.x + pad, y, rect.w - pad * 2.0, line);
        row(ctx, at, 0.0, text, style);
        y += line;
    }
    rect
}

/// Each line of `text` cut into pieces of `cols` characters.
fn wrapped(text: &str, cols: usize) -> Vec<String> {
    text.lines()
        .flat_map(|line| {
            let chars: Vec<char> = line.chars().collect();
            match chars.is_empty() {
                true => vec![String::new()],
                false => chars
                    .chunks(cols)
                    .map(|piece| piece.iter().collect())
                    .collect(),
            }
        })
        .collect()
}
