//! The panel's lines laid out as rows: prose wrapped by words within the widest it may be.

use groove_types::Highlight;

use super::{Piece, Said};
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::base::tokens::POPUP_LINES;

pub(super) enum Row {
    Prose {
        pieces: Vec<Piece>,
        role: Role,
        mark: Option<Mark>,
    },
    Code {
        text: String,
        spans: Vec<Highlight>,
    },
    Rule,
    Gap,
}

/// A row and how wide it draws.
pub(super) struct Laid {
    pub(super) row: Row,
    pub(super) width: f32,
}

/// The rows of every line, `POPUP_LINES` at most, a rule never first or last.
pub(super) fn rows<A: App>(ctx: &mut Ctx<'_, A>, said: &[Said], most: f32) -> Vec<Laid> {
    let mut out = Vec::new();
    for line in said {
        match line {
            Said::Rule | Said::Gap => out.push(Laid {
                row: match line {
                    Said::Rule => Row::Rule,
                    _ => Row::Gap,
                },
                width: 0.0,
            }),
            Said::Code { text, spans } => {
                let width = ctx
                    .measure(text, &ctx.styles.prose_code(Role::Text))
                    .min(most);
                let row = Row::Code {
                    text: text.clone(),
                    spans: spans.clone(),
                };
                out.push(Laid { row, width });
            }
            Said::Prose { pieces, role, mark } => {
                out.extend(wrapped(ctx, pieces, (*role, *mark), most))
            }
        }
    }
    out.truncate(POPUP_LINES);
    while matches!(out.last().map(|one| &one.row), Some(Row::Rule)) {
        out.pop();
    }
    if matches!(out.first().map(|one| &one.row), Some(Row::Rule)) {
        out.remove(0);
    }
    out
}

/// One prose line cut at its spaces into rows no wider than `most`; the mark on the first.
fn wrapped<A: App>(
    ctx: &mut Ctx<'_, A>,
    pieces: &[Piece],
    (role, mark): (Role, Option<Mark>),
    most: f32,
) -> Vec<Laid> {
    let lead = mark.map_or(0.0, |_| ctx.tokens.small + ctx.tokens.xs);
    let mut out: Vec<Laid> = Vec::new();
    let (mut row, mut width) = (Vec::<Piece>::new(), lead);
    for piece in pieces {
        let style = super::styled(ctx, piece.code, role);
        for word in piece.text.split_inclusive(' ') {
            let wide = ctx.measure(word, &style);
            if width + wide > most && !row.is_empty() {
                let first = out.is_empty();
                out.push(laid(std::mem::take(&mut row), (role, mark, first), width));
                width = lead;
            }
            match row.last_mut().filter(|last| last.code == piece.code) {
                Some(last) => last.text.push_str(word),
                None => row.push(Piece {
                    text: word.to_string(),
                    code: piece.code,
                }),
            }
            width += wide;
        }
    }
    let first = out.is_empty();
    out.push(laid(row, (role, mark, first), width));
    out
}

fn laid(pieces: Vec<Piece>, (role, mark, first): (Role, Option<Mark>, bool), width: f32) -> Laid {
    let mark = mark.filter(|_| first);
    Laid {
        row: Row::Prose { pieces, role, mark },
        width,
    }
}
