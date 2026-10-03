//! The lines held at the top of the surface: the file, and the scopes around it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{At, Document, shown};
use groove_gfx::Rect;
use groove_types::Highlight;

use super::notes::Inline;
use crate::Ui;
use crate::components::{Gutters, Line, Rows, code, first, head_mark};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Face;
use groove_ui_kit::base::tokens::{PINNED_DEEP, PINNED_SHARE};

/// One line held above the rows.
struct Pin {
    number: String,
    text: String,
    spans: Vec<Highlight>,
    kind: Kind,
}

/// What a pin stands for: the file, or a scope around the rows.
#[derive(PartialEq)]
enum Kind {
    Head,
    Scope,
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, gutters: Gutters) {
    let pins = held(ctx, body, app, ui);
    if pins.is_empty() {
        return;
    }
    let numbers: Vec<Vec<&str>> = pins.iter().map(|pin| vec![pin.number.as_str()]).collect();
    let height = ctx.tokens.line * pins.len() as f32;
    let band = Rect::new(body.x, body.y, body.w, height);
    let ground = ctx.styles.raised();
    let named = pins.iter().find(|pin| pin.kind == Kind::Head);
    let read = named.is_some_and(|pin| super::row::is_read(app, &pin.text));
    let lines: Vec<Line<'_>> = pins
        .iter()
        .enumerate()
        .map(|(at, pin)| match pin.kind {
            Kind::Head => Line::head(&pin.text).read(read),
            Kind::Scope => Line::new(&pin.text)
                .gutters(&numbers[at])
                .spans(&pin.spans)
                .ground(ground),
        })
        .collect();
    ctx.layer();
    let drawn = code(
        ctx,
        band,
        Rows {
            lines: &lines,
            first: 0,
            gutters,
            across: ui.session.across(),
        },
        0.0,
    );
    ctx.hit(band, Target::Pinned);
    let named = pins.iter().position(|pin| pin.kind == Kind::Head);
    if let Some((line, at)) = named.and_then(|at| Some((drawn.get(at)?, at))) {
        let path = pins[at].text.clone();
        ctx.hit(*line, Target::Head(path.clone()));
        ctx.hit(head_mark(ctx, *line), Target::Read(path));
    }
}

/// The scopes around the first row the pinned band leaves showing.
fn held(ctx: &Ctx, body: Rect, app: &AppState, ui: &Ui) -> Vec<Pin> {
    let inline = super::notes::Inline::of(app, ui, ui.session.face());
    if ui.session.face() == Face::File {
        return scoped(ctx, body, app, ui, &inline);
    }
    let mut pins = Vec::new();
    for _ in 0..=PINNED_DEEP {
        let next = pins_under(ctx, body, app, ui, &inline, pins.len());
        if next.len() <= pins.len() {
            return next;
        }
        pins = next;
    }
    pins
}

/// The pins for the row that stands `under` rows below the top of the surface.
fn pins_under(
    ctx: &Ctx,
    body: Rect,
    app: &AppState,
    ui: &Ui,
    inline: &super::notes::Inline,
    under: usize,
) -> Vec<Pin> {
    let rows = inline.total(super::row::count(app, ui.session.face()));
    let at = (first(ctx.tokens.line, ui.session.scroll()) + under).min(rows.saturating_sub(1));
    let top = inline.base(at);
    let Some(path) = standing(app, top) else {
        return Vec::new();
    };
    let mut pins: Vec<Pin> = Vec::new();
    pins.push(said(&path, Kind::Head));
    pins.truncate(room(ctx, body));
    pins
}

/// One pin that names something rather than holding a line of code.
fn said(text: &str, kind: Kind) -> Pin {
    Pin {
        number: String::new(),
        text: text.to_string(),
        spans: Vec::new(),
        kind,
    }
}

/// The open file's scopes whose opening lines have gone above the slot each would stand in.
fn scoped(ctx: &Ctx, body: Rect, app: &AppState, ui: &Ui, inline: &Inline) -> Vec<Pin> {
    let Some(doc) = app.workspace.active().map(|open| open.new.document()) else {
        return Vec::new();
    };
    let (line, scroll) = (ctx.tokens.line, ui.session.scroll());
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
    ((body.h / PINNED_SHARE / ctx.tokens.line) as usize).max(1)
}

/// The file of the row the top of the surface shows.
fn standing(app: &AppState, top: usize) -> Option<String> {
    match app.workspace.changes.at(top)? {
        At::Row(file, _) => Some(file.path.clone()),
        At::Head(_) => above(app, top),
    }
}

/// The file the row above a head row belongs to.
fn above(app: &AppState, top: usize) -> Option<String> {
    let before = app.workspace.changes.at(top.checked_sub(1)?)?;
    let (At::Row(file, _) | At::Head(file)) = before;
    Some(file.path.clone())
}

fn pin(doc: &Document, at: usize) -> Option<Pin> {
    let text = doc.line(at)?.to_string();
    let width = doc.indent().width();
    let (text, spans) = shown(&text, &doc.spans(at), width);
    Some(Pin {
        number: (at + 1).to_string(),
        text,
        spans,
        kind: Kind::Scope,
    })
}
