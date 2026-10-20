//! The panel's lines as rows no wider than the panel: prose cut at words, code at characters.

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
        if out.len() >= POPUP_LINES {
            break;
        }
        match line {
            Said::Rule | Said::Gap => out.push(Laid {
                row: match line {
                    Said::Rule => Row::Rule,
                    _ => Row::Gap,
                },
                width: 0.0,
            }),
            Said::Code { text, spans } => out.extend(folded(ctx, text, spans, most)),
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
        let fits = room(ctx, &style, most - lead);
        let words = piece.text.split_inclusive(' ');
        for word in words.flat_map(|word| cut(word, fits)) {
            if out.len() >= POPUP_LINES {
                break;
            }
            let word = word.as_str();
            let wide = ctx.measure(word, &style);
            if width + wide > most && !row.is_empty() {
                let first = out.is_empty();
                out.push(laid(
                    std::mem::take(&mut row),
                    (role, mark, first),
                    (width, most),
                ));
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
    out.push(laid(row, (role, mark, first), (width, most)));
    out
}

fn laid(
    pieces: Vec<Piece>,
    (role, mark, first): (Role, Option<Mark>, bool),
    (width, most): (f32, f32),
) -> Laid {
    let mark = mark.filter(|_| first);
    Laid {
        row: Row::Prose { pieces, role, mark },
        width: width.min(most),
    }
}

/// How many characters of `style` fit in `most`, one at least.
fn room<A: App>(ctx: &mut Ctx<'_, A>, style: &groove_gfx::TextStyle, most: f32) -> usize {
    let one = ctx.measure("0", style).max(1.0);
    ((most / one).floor() as usize).max(1)
}

/// `text` in pieces of `room` characters at most.
fn cut(text: &str, room: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.chunks(room).map(|one| one.iter().collect()).collect()
}

/// A code line in rows of the characters that fit, each keeping its spans.
fn folded<A: App>(ctx: &mut Ctx<'_, A>, text: &str, spans: &[Highlight], most: f32) -> Vec<Laid> {
    let style = ctx.styles.prose_code(Role::Text);
    let mut out = Vec::new();
    let mut start = 0;
    for piece in cut(text, room(ctx, &style, most)) {
        let end = start + piece.len();
        let spans = groove_types::spans_within(spans, start..end);
        let width = ctx.measure(&piece, &style).min(most);
        out.push(Laid {
            row: Row::Code { text: piece, spans },
            width,
        });
        start = end;
    }
    out
}
