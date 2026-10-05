//! The file view keeps the caret in sight once a key has moved it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Document, display_of};
use groove_types::Caret;

use crate::hit::{Hits, Target};
use crate::views::session::Face;
use crate::views::session::diff::{Inline, text_at};
use crate::{Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// Scrolls the file view to a caret that moved since the last call, on both axes.
pub fn follow(ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
    let Some(caret) = caret_of(ui, app) else {
        return;
    };
    if ui.session.followed.replace(caret) == Some(caret) {
        return;
    }
    let Some(rect) = hits.rect_of(&Target::Code) else {
        return;
    };
    let tokens = metrics.tokens();
    let row = Inline::of(app, ui, Face::File, hits.wrap()).shifted(caret.line) as f32 * tokens.line;
    *ui.session.scroll_mut() = into_view(ui.session.scroll(), row, tokens.line, rect.h);
    if let Some(file) = app.workspace.active() {
        let to = across_at(ui, app, (&file.path, caret), (hits, metrics));
        *ui.session.across_mut() = to;
    }
}

/// The least change to the sideways offset that shows `at` of the file at `path`.
pub(crate) fn across_at(
    ui: &Ui,
    app: &AppState,
    (path, at): (&str, Caret),
    (hits, metrics): (&Hits, Metrics),
) -> f32 {
    let across = ui.session.across();
    match text_at(app, path, at.line) {
        Some((text, width)) => {
            across_to(across, display_of(&text, at.column, width), hits, metrics)
        }
        None => across,
    }
}

/// The sideways offset that shows column `at` of `text`, from a file view at its left edge.
pub(crate) fn reaching(text: &str, path: &str, at: usize, hits: &Hits, metrics: Metrics) -> f32 {
    let width = Document::plain(path, text).indent().width();
    across_to(0.0, display_of(text, at, width), hits, metrics)
}

/// The least change to `across` that shows display column `column`.
fn across_to(across: f32, column: usize, hits: &Hits, metrics: Metrics) -> f32 {
    let Some(rect) = hits.rect_of(&Target::Code) else {
        return across;
    };
    let chars = hits.chars();
    let room = rect.right() - (chars.left + across) - metrics.tokens().md;
    into_view(across, column as f32 * chars.advance, chars.advance, room)
}

/// The caret of the file view, while it holds the keyboard.
fn caret_of(ui: &Ui, app: &AppState) -> Option<Caret> {
    let here = ui.focus == Focus::Workspace && !ui.session.typing();
    let file = app.workspace.active().filter(|_| here)?;
    (ui.session.face() == Face::File).then(|| file.new.caret())
}

/// The least offset change that shows `[at, at + size)` in a window of `room`.
fn into_view(offset: f32, at: f32, size: f32, room: f32) -> f32 {
    if at < offset {
        return at;
    }
    if at + size > offset + room {
        return (at + size - room).max(0.0);
    }
    offset
}
