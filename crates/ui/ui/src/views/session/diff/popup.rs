//! What the language servers say of one spot of the open file, in a panel under it.

mod said;

use std::cell::RefCell;
use std::rc::Rc;

use groove_controllers::AppState;
use groove_controllers::workspace_service::Hovered;
use groove_gfx::Rect;
use groove_types::Caret;
use groove_ui_kit::base::tokens::REST_MS;
use groove_ui_kit::widgets::{Said, popup};

use crate::ctx::Ctx;
use crate::{Focus, Ui};

/// A spot of an open file the pointer stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointed {
    pub path: String,
    pub at: Caret,
}

/// Under the caret's column of `line`, for a popup asked from the keyboard to stand at.
pub(super) fn anchor(ctx: &mut Ctx, line: Rect, column: usize) {
    let chars = ctx.app.hits.chars();
    let x = chars.left + column as f32 * chars.advance;
    ctx.app.hits.caret_at((x, line.bottom()));
}

/// The popup, while the spot it speaks of is still where the pointer or the caret rests.
pub(crate) fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(hovered) = app.workspace.servers.hovered.as_ref() else {
        return;
    };
    let Some(at) = place(ctx, app, ui, hovered) else {
        return;
    };
    let said = said::said(app, ui, hovered);
    if !said.is_empty() {
        popup(ctx, at, &said);
    }
}

/// Under the caret while it stays on the spot, else under the pointer once it has rested there.
fn place(ctx: &Ctx, app: &AppState, ui: &Ui, hovered: &Hovered) -> Option<(f32, f32)> {
    if hovered.caret {
        let open = app.workspace.active()?;
        let here = open.path == hovered.path && open.new.caret() == hovered.at;
        return (here && ui.focus == Focus::Workspace).then(|| ctx.app.hits.caret())?;
    }
    let here = Pointed {
        path: hovered.path.clone(),
        at: hovered.at,
    };
    let below = (ui.at.0, ui.at.1 + ctx.tokens.line);
    if hovered.told {
        return (ui.session.pointed.as_ref() == Some(&here)).then_some(below);
    }
    let (still, since) = ui.session.pointer_rest.as_ref()?;
    let asked = still.path == hovered.path && still.at == hovered.at;
    let rested = ctx.tick >= since + REST_MS && ui.session.pointed.as_ref() == Some(still);
    (asked && rested).then_some(below)
}

/// The file and the answer the lines were laid out for.
type Key = (String, String);

/// The hover's lines as last laid out, kept while the answer and its file stay the same.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Spoken {
    kept: RefCell<Option<(Key, Rc<Vec<Said>>)>>,
}

impl Spoken {
    pub(super) fn of(&self, key: (&str, &str), build: impl FnOnce() -> Vec<Said>) -> Rc<Vec<Said>> {
        let mut kept = self.kept.borrow_mut();
        if let Some(((path, text), lines)) = kept.as_ref()
            && (path.as_str(), text.as_str()) == key
        {
            return lines.clone();
        }
        let lines = Rc::new(build());
        *kept = Some(((key.0.to_string(), key.1.to_string()), lines.clone()));
        lines
    }
}
