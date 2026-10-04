//! One row drawn: a note's own rows here, every other row through the kit.

mod note;

use self::note::{acts, note};

use groove_gfx::Rect;
use groove_ui_kit::widgets::code::{self, Block, Code};

use super::Line;
use crate::ctx::Ctx;
use crate::hit::Target;

pub(super) fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block, across: f32) {
    if let Some(said) = code.said {
        return note(ctx, line, code.text, said, gutter);
    }
    if let Some(acting) = code.acting.as_ref() {
        return acts(ctx, line, acting, gutter);
    }
    code::draw(ctx, line, &painted(code), gutter, across);
}

fn painted<'a>(line: &Line<'a>) -> Code<'a, Target> {
    let blame = line.blame.map(|(said, sha)| {
        let opens = sha.map(|sha| Target::Blamed(sha.to_string()));
        (said, opens)
    });
    Code {
        gutters: line.gutters,
        text: line.text,
        spans: line.spans,
        ground: line.ground,
        mark: line.mark,
        banner: line.banner,
        head: line.head,
        folded: line.folded,
        read: line.read,
        caret: line.caret,
        held: line.held,
        found: line.found,
        words: line.words,
        word: line.word,
        standing: line.standing,
        noted: line.noted,
        blame,
    }
}
