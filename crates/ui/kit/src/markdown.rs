//! Markdown prose: parsed into blocks, laid out to a width, drawn with its links.

mod compact;
mod draw;
pub mod layout;
pub mod parse;
#[cfg(test)]
mod tests;

use groove_gfx::Rect;

pub use compact::{Row, compact, row};
pub use draw::draw;
pub use layout::{Laid, laid};
pub use parse::blocks;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// The first block's text, without its marks.
pub fn plain(text: &str) -> String {
    let blocks = blocks(text);
    let first = blocks.iter().find(|one| !one.spans.is_empty());
    let words = first.map(|one| one.spans.iter().map(|span| span.text.as_str()));
    words
        .into_iter()
        .flatten()
        .collect::<String>()
        .trim()
        .to_string()
}

/// `text` from the top of `at`, `role` for its paragraphs; returns the height it stands.
pub fn prose<A: App>(
    ctx: &mut Ctx<'_, A>,
    (at, visible): (Rect, Rect),
    text: &str,
    role: Role,
    link: impl Fn(&str) -> A::Target,
) -> f32 {
    let laid = laid(ctx, &blocks(text), at.w, role);
    draw(ctx, at, visible, &laid, &link);
    laid.height
}
