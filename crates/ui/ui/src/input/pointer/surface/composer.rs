//! What the composer of a note does: its label and decorations written into the words, sent, or dropped.

use groove_controllers::Command;
use groove_types::{Caret, Edit, Labelled, Motion, labelled};

use crate::Ui;
use crate::hit::Composing;

/// One of the composer's chips or buttons, clicked.
pub(crate) fn composing(ui: &mut Ui, asked: Composing) -> Vec<Command> {
    match asked {
        Composing::Save => {
            let act = ui.session.noting.take().and_then(|noting| noting.sent());
            act.map(Command::Delivery).into_iter().collect()
        }
        Composing::Cancel => {
            ui.session.noting = None;
            Vec::new()
        }
        Composing::Label(label) => prefixed(ui, |held| match held {
            Some((was, _)) if was == label => None,
            Some((_, decorations)) => Some((label, decorations)),
            None => Some((label, Vec::new())),
        }),
        Composing::Decoration(decoration) => prefixed(ui, |held| {
            let (label, mut decorations) = held?;
            match decorations.iter().position(|one| *one == decoration) {
                Some(at) => drop(decorations.remove(at)),
                None => {
                    decorations.retain(|one| !["blocking", "non-blocking"].contains(one));
                    decorations.push(decoration);
                }
            }
            Some((label, decorations))
        }),
    }
}

type Prefix<'a> = Option<(&'a str, Vec<&'a str>)>;

/// The words' `label (decorations): ` written again from what `change` makes of the one there.
fn prefixed(ui: &mut Ui, change: impl FnOnce(Prefix<'_>) -> Prefix<'_>) -> Vec<Command> {
    let Some(noting) = ui.session.noting.as_mut() else {
        return Vec::new();
    };
    let text = noting.buffer.text();
    let held = labelled(&text);
    let old = held.as_ref().map_or(0, |one| text.len() - one.rest.len());
    let old = text[..old].chars().count();
    let held = held.map(|one| (one.label, one.decorations));
    let new = change(held).map_or_else(String::new, |(label, decorations)| {
        Labelled::prefix(label, &decorations)
    });
    let caret = noting.buffer.caret();
    let column = match caret.line {
        0 => (caret.column + new.chars().count()).saturating_sub(old),
        _ => caret.column,
    };
    let buffer = &mut noting.buffer;
    buffer.edit(&Edit::Move(Motion::To(Caret::new(0, 0))));
    buffer.edit(&Edit::Extend(Motion::To(Caret::new(0, old))));
    buffer.edit(&Edit::Insert(new));
    buffer.edit(&Edit::Move(Motion::To(Caret::new(caret.line, column))));
    Vec::new()
}
