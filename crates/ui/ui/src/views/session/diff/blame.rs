//! What the blame says after the line the caret rests on.

use groove_controllers::AppState;
use groove_types::{BlameLine, Timestamp};
use groove_ui_kit::base::tokens::REST_MS;
use groove_ui_kit::text::ago;

use super::row::Drawn;
use crate::ctx::Ctx;
use crate::views::session::Tab;
use crate::{Focus, Ui};

/// A line of the open buffer, at one revision of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub path: String,
    pub line: usize,
    pub revision: u64,
}

/// What the blame says, and the commit a click on it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub text: String,
    pub sha: Option<String>,
}

/// The line the caret stands on, while the code surface holds the keyboard.
pub(crate) fn spot(ui: &Ui, app: &AppState) -> Option<Spot> {
    let here = ui.focus == Focus::Workspace && !ui.session.typing();
    let code = matches!(ui.session.tab, Tab::Diff | Tab::Files);
    if !here || !code || app.workspace.readonly() {
        return None;
    }
    let file = app.workspace.active()?;
    Some(Spot {
        path: file.path.clone(),
        line: file.new.caret().line,
        revision: file.new.revision(),
    })
}

/// The spot once the caret has rested on it long enough, at frame tick `tick`.
pub(crate) fn rested(ui: &Ui, app: &AppState, tick: u64) -> Option<Spot> {
    let (at, since) = ui.session.rest.as_ref()?;
    let still = spot(ui, app).as_ref() == Some(at);
    (still && tick >= since + REST_MS).then(|| at.clone())
}

/// What the blame of the line the caret rests on says, once it is read.
pub(super) fn said(app: &AppState, ui: &Ui, (tick, now): (u64, Timestamp)) -> Option<Said> {
    let at = rested(ui, app, tick)?;
    let worktree = app.session.selected_worktree()?;
    let read = app.workspace.read_of(&at.path);
    let lines = app.workspace.blames.of(&worktree.id, &at.path, read)?;
    Some(match lines.get(at.line) {
        Some(line) if !line.uncommitted => Said {
            text: words(line, now),
            sha: Some(line.sha.clone()),
        },
        _ => Said {
            text: "You · not committed yet".into(),
            sha: None,
        },
    })
}

/// What a surface says after the caret's line; the old side of a split says nothing.
pub(super) fn shown(ctx: &Ctx, app: &AppState, ui: &Ui, new_side: bool) -> Option<Said> {
    new_side.then(|| said(app, ui, (ctx.tick, ctx.now)))?
}

/// The blame the caret's row carries.
pub(super) fn on<'a>(row: &Drawn, said: Option<&'a Said>) -> Option<(&'a str, Option<&'a str>)> {
    let said = said.filter(|_| row.caret.is_some())?;
    Some((said.text.as_str(), said.sha.as_deref()))
}

/// `Author, 5d ago · a1b2c3d`.
fn words(line: &BlameLine, now: Timestamp) -> String {
    let when = match ago(line.at.age_at(now)).as_str() {
        "now" => "now".to_string(),
        age => format!("{age} ago"),
    };
    format!("{}, {when} · {}", line.author, line.short_sha)
}
