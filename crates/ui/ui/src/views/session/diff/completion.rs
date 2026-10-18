//! What the language servers offer to type at the caret, in a list under it.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{matching, word_start};
use groove_types::{Caret, Completion, Edit, Motion};
use groove_ui_kit::widgets::{Offer, completions};

use crate::ctx::Ctx;
use crate::views::session::Tab;
use crate::{Focus, Ui};

/// The offer standing at the caret, narrowed to the word typed before it.
pub(crate) struct Shown<'a> {
    pub items: Vec<&'a Completion>,
    pub caret: Caret,
    /// Where the typed word starts.
    pub word: Caret,
    pub chosen: usize,
    pub asked: u64,
}

/// The offer at the caret of the open file while the keyboard is on it; none once nothing matches.
pub(crate) fn shown<'a>(app: &'a AppState, ui: &Ui) -> Option<Shown<'a>> {
    let offered = app.workspace.servers.offered.as_ref()?;
    let code = matches!(ui.session.tab, Tab::Diff | Tab::Files);
    let open = app.workspace.active()?;
    let caret = open.new.caret();
    let here = open.path == offered.path && caret == offered.at;
    if !(here && code && ui.focus == Focus::Workspace) {
        return None;
    }
    let (line, _) = super::text_at((app, ui), &open.path, caret.line)?;
    let start = word_start(&line, caret.column);
    let typed: String = line
        .chars()
        .skip(start)
        .take(caret.column - start)
        .collect();
    let items = matching(&offered.items, &typed);
    let done = items.len() == 1 && items[0].insert == typed;
    if items.is_empty() || done {
        return None;
    }
    let (asked, chosen) = ui.session.chosen;
    let chosen = match asked == offered.asked {
        true => chosen.min(items.len() - 1),
        false => 0,
    };
    Some(Shown {
        items,
        caret,
        word: Caret::new(caret.line, start),
        chosen,
        asked: offered.asked,
    })
}

/// The edits that type the chosen item over what it replaces, the caret's side of it included.
pub(crate) fn accepted(shown: &Shown) -> Vec<Edit> {
    let item = shown.items[shown.chosen];
    let (from, to) = item.range.unwrap_or((shown.word, shown.caret));
    let (from, to) = (from.min(shown.caret), to.max(shown.caret));
    let mut edits = Vec::new();
    if to != shown.caret {
        edits.push(Edit::Move(Motion::To(to)));
    }
    if from != to {
        edits.push(Edit::Extend(Motion::To(from)));
    }
    edits.push(Edit::Insert(item.insert.clone()));
    edits
}

/// The list under the caret, while an offer stands there.
pub(crate) fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let (Some(shown), Some(at)) = (shown(app, ui), ctx.app.hits.caret()) else {
        return;
    };
    let offers: Vec<Offer> = shown
        .items
        .iter()
        .map(|one| Offer {
            label: &one.label,
            detail: one.detail.as_deref().unwrap_or_default(),
        })
        .collect();
    completions(ctx, at, &offers, shown.chosen);
}
