//! What a note row says: the words, who said them, and the lines they are about.

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines};
use super::wrap::Worded;
use crate::Ui;
use crate::components::{Composed, Field, Header};
use crate::views::session::{Noting, Writing};
use groove_controllers::delivery_service::BY_USER;

#[derive(Default)]
pub(super) struct Words {
    pub(super) author: String,
    pub(super) lines: String,
    pub(super) body: String,
    pub(super) prose: Option<groove_ui_kit::markdown::Row>,
    pub(super) head: Option<Header>,
    /// A resolved note shown as one line.
    pub(super) folded: bool,
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
            Some(noting) => typed(noting, row, ui.focus == crate::Focus::Workspace),
            None => Words::default(),
        },
        Slot::Folded { at } => match app.delivery.shown.get(at) {
            Some(note) => Words {
                head: Some(header(note, 0)),
                body: subject(note),
                folded: true,
                ..Words::default()
            },
            None => Words::default(),
        },
        Slot::Acts { .. } | Slot::Pad { .. } | Slot::Code(_) => Words::default(),
    }
}

/// One row of the note being typed: its line of the buffer, the caret and what is held on it.
fn typed(noting: &Noting, row: usize, lit: bool) -> Words {
    let buffer = &noting.buffer;
    let body = buffer.line(row).unwrap_or_default().to_string();
    let caret = buffer.caret();
    let chars = body.chars().count();
    let last = row + 1 == buffer.text().split('\n').count();
    Words {
        field: Some(Field {
            first: row == 0,
            last,
            lit,
        }),
        held: buffer
            .selections()
            .first()
            .and_then(|one| one.on(row, chars)),
        caret: (caret.line == row).then_some(caret.column),
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

/// The header of a note's comment `at`: who, from where, its label, its lines, when.
fn header(note: &groove_types::Note, at: usize) -> Header {
    let Some(said) = note.said.get(at) else {
        return Header::default();
    };
    let label = groove_types::labelled(&said.body).map(|one| {
        (
            one.label.to_string(),
            one.decorations.iter().map(|one| one.to_string()).collect(),
        )
    });
    let author = match said.author.as_str() {
        BY_USER => "me",
        other => other,
    };
    Header {
        initials: initials(author),
        author: author.to_string(),
        source: if note.is_local() { "local" } else { "MR" },
        label,
        lines: match at {
            0 => note.anchor.as_ref().map(lines).unwrap_or_default(),
            _ => String::new(),
        },
        at: said.at,
        reply: at > 0,
        replies: note.replies(),
        folds: (note.resolved && at == 0).then(|| note.origin.clone()),
    }
}

/// The first line a note opens with, its label off.
fn subject(note: &groove_types::Note) -> String {
    let body = note.opening().map_or("", |said| said.body.as_str());
    let words = groove_types::labelled(body).map_or(body, |one| one.rest);
    words.lines().next().unwrap_or_default().to_string()
}

/// Two capitals for a name: its first two words' first letters, or a lone word's first two.
fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|one: char| !one.is_alphanumeric())
        .filter(|one| !one.is_empty())
        .collect();
    let letters: String = match words.as_slice() {
        [one] => one.chars().take(2).collect(),
        many => many
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect(),
    };
    letters.to_uppercase()
}
