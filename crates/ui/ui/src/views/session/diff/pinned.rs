//! The lines held at the top of the surface: the scopes around the open file's rows.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Document, shown};
use groove_gfx::Rect;
use groove_types::Highlight;

use super::notes::Inline;
use crate::Ui;
use crate::components::{Gutters, Line, Rows, code};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Face;
use groove_ui_kit::base::tokens::{PINNED_DEEP, PINNED_SHARE};
use groove_ui_kit::shape::shadow_under;
use groove_ui_kit::widgets::glass_pane;

/// One line held above the rows: a scope around them.
#[derive(Clone)]
struct Pin {
    number: String,
    text: String,
    spans: Vec<Highlight>,
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    ui: &Ui,
    (inline, gutters): (&Inline, Gutters),
) {
    let pins = match ui.session.face() {
        Face::File => scoped(ctx, body, app, ui, inline),
        Face::Stream(_) => Vec::new(),
    };
    if pins.is_empty() {
        return;
    }
    let numbers: Vec<Vec<&str>> = pins.iter().map(|pin| vec![pin.number.as_str()]).collect();
    let height = ctx.tokens.code_line * pins.len() as f32;
    let band = groove_ui_kit::shape::band(body, 0.0, height);
    let ground = ctx.styles.glass_card();
    let lines: Vec<Line<'_>> = pins
        .iter()
        .enumerate()
        .map(|(at, pin)| {
            Line::new(&pin.text)
                .gutters(&numbers[at])
                .spans(&pin.spans)
                .ground(ground)
        })
        .collect();
    ctx.layer();
    ctx.clipped(body, |ctx| {
        backed(ctx, band);
        let rows = Rows {
            lines: &lines,
            first: 0,
            gutters,
            across: ui.session.across(),
            floating: false,
        };
        code(ctx, band, rows, 0.0);
        ctx.hit(band, Target::Pinned);
    });
}

/// The glass the pins stand on, across the surface whole.
fn backed(ctx: &mut Ctx, band: Rect) {
    shadow_under(ctx, band);
    glass_pane(ctx, band);
}

/// The open file's scopes whose opening lines have gone above the slot each would stand in.
fn scoped(ctx: &Ctx, body: Rect, app: &AppState, ui: &Ui, inline: &Inline) -> Vec<Pin> {
    let Some(doc) = super::row::edited(app, ui).map(|one| one.buffer.document()) else {
        return Vec::new();
    };
    let (line, scroll) = (ctx.tokens.code_line, ui.session.scroll());
    let last = inline.total(doc.lines().max(1)).saturating_sub(1);
    let mut held: Vec<usize> = Vec::new();
    for _ in 0..=PINNED_DEEP {
        let below = ((scroll + held.len() as f32 * line) / line).ceil() as usize;
        let chain = doc.scopes(inline.base(below.min(last)));
        let chain = &chain[chain.len().saturating_sub(PINNED_DEEP)..];
        let gone = |(slot, start): &(usize, &usize)| {
            (inline.shifted(**start) as f32) * line < scroll + *slot as f32 * line
        };
        let next: Vec<usize> = chain
            .iter()
            .enumerate()
            .take_while(gone)
            .map(|(_, at)| *at)
            .collect();
        let next = next[..next.len().min(room(ctx, body))].to_vec();
        if next.len() <= held.len() {
            held = next;
            break;
        }
        held = next;
    }
    held.into_iter().filter_map(|at| pin(doc, at)).collect()
}

/// How many lines the surface will give up to what stands above it.
fn room(ctx: &Ctx, body: Rect) -> usize {
    ((body.h / PINNED_SHARE / ctx.tokens.code_line) as usize).max(1)
}

fn pin(doc: &Document, at: usize) -> Option<Pin> {
    let text = doc.line(at)?.to_string();
    let width = doc.indent().width();
    let (text, spans) = shown(&text, &doc.spans(at), width);
    Some(Pin {
        number: (at + 1).to_string(),
        text,
        spans,
    })
}
