//! One row drawn: a note's own rows here, every other row through the kit.

mod composer;
mod note;

use self::composer::composer;
use self::note::{acts, note, padded};

use groove_gfx::Rect;
use groove_ui_kit::widgets::code::{self, Block, Code};

use super::Line;
use crate::ctx::Ctx;
use crate::hit::{NoteButton, Target};

pub(super) fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block, across: f32) {
    if code.covered {
        return;
    }
    if let Some(composed) = code.composed.as_ref() {
        return composer(ctx, line, composed, (gutter, code.edges));
    }
    if let Some(room) = code.pad {
        return padded(ctx, line, room, (gutter, code.edges));
    }
    if let Some(said) = code.said {
        return note(ctx, line, code, said, gutter);
    }
    if let Some(acting) = code.acting.as_ref() {
        return acts(ctx, line, acting, (gutter, code.edges));
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
        gutter: line.gutter,
        mark: line.mark,
        banner: line.banner,
        head: line.head,
        folded: line.folded,
        read: line.read,
        part: line.part,
        counts: line.counts,
        tile: line.tile.as_ref().map(|(tile, folds)| {
            let opens = folds
                .clone()
                .map(|origin| Target::Note(origin, NoteButton::Fold));
            (*tile, opens)
        }),
        empty: line.empty,
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
