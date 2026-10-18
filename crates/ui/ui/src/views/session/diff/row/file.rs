//! The file view's rows: the buffer's lines as they stand now, wrapped or folded.

use std::ops::Range;

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Block, display_of};
use groove_types::RowKind;

use super::{
    Drawn, Outline, caret, columns_in, edited, held, levels, matched, occurs, shown, standing,
    text_of,
};
use crate::Ui;
use crate::views::session::diff::blocks;

/// The rows in `window`: the buffer's lines, or their pieces while the lines wrap.
pub(super) fn whole(app: &AppState, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let Some((starts, cols)) = super::super::wrapping::wrap_of(app, ui) else {
        return lines(app, ui, window);
    };
    let first = super::super::wrapping::line_of(&starts, window.start).0;
    let last = super::super::wrapping::line_of(&starts, window.end.saturating_sub(1)).0;
    let rows = lines(app, ui, first..last + 1);
    window
        .filter_map(|row| {
            let (line, part) = super::super::wrapping::line_of(&starts, row);
            let whole = rows.get(line - first)?;
            Some(super::super::wrapping::piece(whole, part, cols))
        })
        .collect()
}

/// The lines of the buffer as it is now, marked where the file's change touched them.
fn lines(app: &AppState, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let Some(editor) = edited(app, ui) else {
        return Vec::new();
    };
    let buffer = editor.buffer;
    let caret = caret(ui, buffer);
    let stamp = (editor.stamp, buffer.painted());
    let doc = buffer.document();
    let colours = ui
        .painted
        .of(doc, &editor.path, false, stamp, window.clone());
    let width = doc.indent().width();
    let hunked = editor.file.map(|file| &file.hunked);
    let (blocks, folded) = (blocks::of(app, ui), blocks::folded(app, ui));
    let word = ui.session.word.as_deref();
    window
        .map(|at| {
            let text = text_of(buffer, at);
            let painted = match editor.levels {
                Some(shape) => levels::spans(&text, shape),
                None => colours.of(at).to_vec(),
            };
            let (drawn, spans) = shown(&text, &painted, width);
            Drawn {
                text: drawn,
                spans,
                gutters: vec![(at + 1).to_string()],
                kind: RowKind::Context,
                caret: caret
                    .filter(|on| on.line == at)
                    .map(|on| display_of(&text, on.column, width)),
                held: held(buffer, ui, at),
                mark: hunked.and_then(|one| one.marks.get(&(at as u32)).copied()),
                words: columns_in(
                    hunked.and_then(|one| one.words.new.get(&(at as u32))),
                    &text,
                    width,
                ),
                found: matched(ui, at, &text, width),
                standing: standing(ui, at, &text, width),
                faults: faulted(app, &editor.path, (at, &text, width)),
                outline: outlined((&blocks, &folded), word, (at, &text, width)),
                ..Drawn::default()
            }
        })
        .collect()
}

/// What the file view draws about line `at` besides its text.
fn outlined(
    (blocks, folded): (&[Block], &[Block]),
    word: Option<&str>,
    (at, text, width): (usize, &str, usize),
) -> Outline {
    Outline {
        occurs: word.map_or_else(Vec::new, |word| occurs(text, word, width)),
        guides: blocks::guides(blocks, at),
        fold: blocks::opens(blocks, folded, at).map(|(shut, hides)| (shut, hides, at)),
    }
}

/// What the servers found on line `at` of `path`, in the columns the row draws.
fn faulted(
    app: &AppState,
    path: &str,
    (at, text, width): (usize, &str, usize),
) -> Vec<super::super::faults::Fault> {
    super::super::faults::on(super::super::faults::of(app, path, at), at, text, width)
}
