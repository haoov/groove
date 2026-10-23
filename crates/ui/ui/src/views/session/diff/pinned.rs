//! The lines held at the top of the surface: the file, and the scopes around it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{At, Document, HEAD_ROWS, shown};
use groove_gfx::{Edges, Rect};
use groove_types::Highlight;

use super::notes::Inline;
use crate::Ui;
use crate::components::{Gutters, Line, Rows, code, first};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Face;
use groove_ui_kit::base::tokens::{PINNED_DEEP, PINNED_SHARE};
use groove_ui_kit::shape::shadow_under;
use groove_ui_kit::widgets::glass_pane;

/// One line held above the rows.
#[derive(Clone)]
struct Pin {
    number: String,
    text: String,
    spans: Vec<Highlight>,
    kind: Kind,
}

/// What a pin stands for: the file, or a scope around the rows.
#[derive(Clone, PartialEq)]
enum Kind {
    Head,
    Scope,
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    ui: &Ui,
    (inline, gutters): (&Inline, Gutters),
) {
    let pins = held(ctx, body, app, ui, inline);
    if pins.is_empty() {
        return;
    }
    let numbers: Vec<Vec<&str>> = pins.iter().map(|pin| vec![pin.number.as_str()]).collect();
    let height = ctx.tokens.code_line * pins.len() as f32;
    let named = pins.iter().find(|pin| pin.kind == Kind::Head);
    let pushed = named.and_then(|pin| pushed(ctx, (app, ui, inline), &pin.text, height));
    let band = groove_ui_kit::shape::band(body, pushed.unwrap_or(0.0), height);
    let ground = ctx.styles.raised();
    let read = named.is_some_and(|pin| super::row::is_read(app, &pin.text));
    let lines: Vec<Line<'_>> = pins
        .iter()
        .enumerate()
        .map(|(at, pin)| match pin.kind {
            Kind::Head if at > 0 => Line::covered(),
            Kind::Head => Line::head(&pin.text)
                .read(read)
                .part((0, pins.len()))
                .counts(super::row::counted(app, &pin.text)),
            Kind::Scope => Line::new(&pin.text)
                .gutters(&numbers[at])
                .spans(&pin.spans)
                .ground(ground),
        })
        .collect();
    ctx.layer();
    ctx.clipped(body, |ctx| {
        backed(ctx, band, named.is_some());
        let rows = Rows {
            lines: &lines,
            first: 0,
            gutters,
            across: ui.session.across(),
        };
        let drawn = code(ctx, band, rows, 0.0);
        ctx.hit(band, Target::Pinned);
        if let Some(pin) = named.filter(|_| !drawn.is_empty()) {
            super::offers::head_hits(ctx, band, &pin.text);
        }
    });
}

/// The glass the pins stand on, shaped as a head's rounded row when they name a file.
fn backed(ctx: &mut Ctx, band: Rect, head: bool) {
    shadow_under(ctx, band);
    let pane = match head {
        true => band.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.xs / 2.0)),
        false => band,
    };
    glass_pane(ctx, pane);
}

/// How far the next file's head, coming up under the pinned one, pushes it up.
fn pushed(
    ctx: &Ctx,
    (app, ui, inline): (&AppState, &Ui, &Inline),
    path: &str,
    height: f32,
) -> Option<f32> {
    let changes = &app.workspace.changes;
    let head = changes.head_of(path)?;
    let (next, _, _) = changes.placed().find(|(start, _, _)| *start > head)?;
    let top = inline.shifted(next) as f32 * ctx.tokens.code_line - ui.session.scroll();
    Some((top - height).min(0.0))
}

/// The scopes around the first row the pinned band leaves showing.
fn held(ctx: &Ctx, body: Rect, app: &AppState, ui: &Ui, inline: &Inline) -> Vec<Pin> {
    if ui.session.face() == Face::File {
        return scoped(ctx, body, app, ui, inline);
    }
    if ui.session.scroll() <= 0.0 {
        return Vec::new();
    }
    file_under(ctx, app, ui, inline, HEAD_ROWS)
        .map(|path| vec![said(&path, Kind::Head); HEAD_ROWS])
        .unwrap_or_default()
}

/// The file of the row that stands `under` rows below the top of the surface.
fn file_under(ctx: &Ctx, app: &AppState, ui: &Ui, inline: &Inline, under: usize) -> Option<String> {
    let rows = inline.total(super::row::count(app, ui, ui.session.face()));
    let at = (first(ctx.tokens.code_line, ui.session.scroll()) + under).min(rows.saturating_sub(1));
    standing(app, inline.base(at))
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

/// The file of the row the top of the surface shows.
fn standing(app: &AppState, top: usize) -> Option<String> {
    match app.workspace.changes.at(top)? {
        At::Row(file, _) => Some(file.path.clone()),
        At::Head(file) => above(app, app.workspace.changes.head_of(&file.path)?),
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
