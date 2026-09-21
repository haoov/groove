//! The lines held at the top of the surface: the file, and the scopes around it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{At, Document, shown};
use groove_gfx::Rect;
use groove_types::{DiffView, Highlight};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::tokens::{PINNED_DEEP, PINNED_SHARE};
use crate::widget::{Gutters, Line, Rows, code, first, head_mark};

/// One line held above the rows.
struct Pin {
    number: String,
    text: String,
    spans: Vec<Highlight>,
    head: bool,
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
    let read = super::row::is_read(app, &pins[0].text);
    let lines: Vec<Line<'_>> = pins
        .iter()
        .enumerate()
        .map(|(at, pin)| match pin.head {
            true => Line::head(&pin.text).read(read),
            false => Line::new(&pin.text)
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
        },
        0.0,
    );
    ctx.hit(band, Target::Pinned);
    if let (Some(line), true) = (drawn.first(), pins[0].head) {
        let path = pins[0].text.clone();
        ctx.hit(*line, Target::Head(path.clone()));
        ctx.hit(head_mark(ctx, *line), Target::Read(path));
    }
}

/// The file and the scopes the top of the surface stands in.
fn held(ctx: &Ctx, body: Rect, app: &AppState, ui: &Ui) -> Vec<Pin> {
    let top = first(ctx.tokens.line, ui.session.diff);
    let Some((path, at)) = standing(app, ui, top) else {
        return Vec::new();
    };
    let mut pins: Vec<Pin> = Vec::new();
    if ui.session.view != DiffView::Editor {
        pins.push(Pin {
            number: String::new(),
            text: path.clone(),
            spans: Vec::new(),
            head: true,
        });
    }
    pins.extend(scopes(app, &path, at));
    pins.truncate(room(ctx, body));
    pins
}

/// The innermost scopes around the line, as lines of their own.
fn scopes(app: &AppState, path: &str, at: Option<(usize, bool)>) -> Vec<Pin> {
    let (Some((line, old)), Some((before, after))) = (at, app.workspace.sides(path)) else {
        return Vec::new();
    };
    let doc = match old {
        true => before,
        false => after,
    };
    let scopes = doc.scopes(line);
    let deep = scopes.len().saturating_sub(PINNED_DEEP);
    scopes[deep..]
        .iter()
        .filter_map(|at| pin(doc, *at))
        .collect()
}

/// How many lines the surface will give up to what stands above it.
fn room(ctx: &Ctx, body: Rect) -> usize {
    ((body.h / PINNED_SHARE / ctx.tokens.line) as usize).max(1)
}

/// The row the top of the surface shows: its file, and the line it stands on.
fn standing(app: &AppState, ui: &Ui, top: usize) -> Option<(String, Option<(usize, bool)>)> {
    if ui.session.view == DiffView::Editor {
        let file = app.workspace.opened.as_ref()?;
        return Some((file.path.clone(), Some((top, false))));
    }
    let At::Row(file, at) = app.workspace.changes.at(top)? else {
        return None;
    };
    let row = &file.rows[at];
    let line = match (row.new, row.old) {
        (Some(line), _) => Some((line as usize, false)),
        (None, Some(line)) => Some((line as usize, true)),
        (None, None) => None,
    };
    Some((file.path.clone(), line))
}

fn pin(doc: &Document, at: usize) -> Option<Pin> {
    let text = doc.line(at)?.to_string();
    let width = doc.indent().width();
    let (text, spans) = shown(&text, &doc.spans(at), width);
    Some(Pin {
        number: (at + 1).to_string(),
        text,
        spans,
        head: false,
    })
}
