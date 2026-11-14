//! What a note row says: the words, who said them, and the lines they are about.

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines};
use super::wrap::Worded;
use crate::Ui;
use crate::components::{Composed, Field, Header};
use crate::views::session::{Noting, Writing};

#[derive(Default)]
pub(super) struct Words {
    pub(super) author: String,
    pub(super) lines: String,
    pub(super) body: String,
    pub(super) prose: Option<groove_ui_kit::markdown::Row>,
    pub(super) head: Option<Header>,
    pub(super) field: Option<Field>,
    pub(super) composed: Option<Composed>,
    /// The caret's column and what is held, on a row of the note being typed.
    pub(super) caret: Option<usize>,
    pub(super) held: Option<(usize, usize, bool)>,
}

/// Who said what on a note row; a row of code says nothing.
pub(super) fn words_of(app: &AppState, ui: &Ui, slot: Slot, inline: &Inline) -> Words {
    match slot {
        Slot::Note { at, row } => match (app.delivery.shown.get(at), inline.said(at, row)) {
            (Some(note), Worded::Head(said)) => Words {
                head: Some(header(note, said)),
                ..Words::default()
            },
            (Some(_), Worded::Body(prose)) => Words {
                prose: Some(prose),
                ..Words::default()
            },
            (None, _) => Words::default(),
        },
        Slot::Chips | Slot::Footer { .. } => Words {
            composed: ui
                .session
                .noting
                .as_ref()
                .map(|noting| composed(noting, slot)),
            ..Words::default()
        },
        Slot::Typed { row, .. } => match ui.session.noting.as_ref() {
            Some(noting) => typed(noting, (row, inline), ui.focus == crate::Focus::Workspace),
            None => Words::default(),
        },
        Slot::Acts { .. } | Slot::Pad { .. } | Slot::Code(_) => Words::default(),
    }
}

/// One row of the note being typed: its piece of a line, the caret and what is held on it.
fn typed(noting: &Noting, (row, inline): (usize, &Inline), lit: bool) -> Words {
    let buffer = &noting.buffer;
    let Some((line, start, end)) = inline.piece(row) else {
        return Words::default();
    };
    let whole = buffer.line(line).unwrap_or_default();
    let chars = whole.chars().count();
    let tail = end >= chars;
    let body: String = whole.chars().skip(start).take(end - start).collect();
    let caret = buffer.caret();
    let on = caret.line == line && caret.column >= start && (caret.column < end || tail);
    let held = buffer
        .selections()
        .first()
        .and_then(|one| one.on(line, chars));
    let held = held.and_then(|(from, to, through)| {
        let (from, to) = (from.max(start), to.min(end));
        (from < to || (from == to && to == end && through && tail)).then_some((
            from - start,
            to - start,
            through && tail,
        ))
    });
    Words {
        field: Some(Field {
            first: row == 0,
            last: row + 1 == inline.typed_rows(),
            lit,
        }),
        held,
        caret: on.then(|| caret.column - start),
        body,
        ..Words::default()
    }
}

/// The composer's label row or its footer, for the note being typed.
fn composed(noting: &Noting, slot: Slot) -> Composed {
    let text = noting.buffer.text();
    let held = groove_types::labelled(&text);
    if let Slot::Footer { reply } = slot {
        let (hint, save) = match noting.writing {
            Writing::New => ("local until you post it", "save note"),
            Writing::Over(_) => ("saves over the note", "save"),
            Writing::Reply(_) => ("", "send"),
        };
        let decorations = held
            .filter(|_| !reply)
            .map(|one| one.decorations.iter().map(|one| one.to_string()).collect());
        return Composed::Footer {
            hint,
            save,
            decorations,
        };
    }
    Composed::Chips {
        lines: lines(&noting.anchor),
        label: held.map(|one| one.label.to_string()),
    }
}

/// The header of a note's comment `at`, its lines on the opening one.
fn header(note: &groove_types::Note, at: usize) -> Header {
    let lines = match at {
        0 => note.anchor.as_ref().map(lines).unwrap_or_default(),
        _ => String::new(),
    };
    Header::of(note, at, lines)
}
