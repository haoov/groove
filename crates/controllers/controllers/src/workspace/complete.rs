//! What the language servers offer to type at the caret, asked again as the user types.

use groove_types::Edit;

use super::servers::clients_of;
use crate::{AppState, Continuation, Services, Spawner};

/// A word character or a dot typed asks again, as a backspace does while an offer stands;
/// any other edit puts the offer away.
pub(super) fn after(state: &mut AppState, spawner: &dyn Spawner, edit: &Edit) {
    let standing = state.workspace.servers.offered.is_some();
    let typing = match edit {
        Edit::Insert(text) => {
            let mut chars = text.chars();
            let one = chars.next().filter(|_| chars.next().is_none());
            one.is_some_and(|c| groove_workspace_service::is_word(c) || c == '.')
        }
        Edit::Backspace => standing,
        _ => false,
    };
    match typing {
        true => ask(state, spawner),
        false if standing => state.workspace.servers.dismiss(),
        false => {}
    }
}

/// Every server of the open file asked at its caret; the answers land while it is the last ask.
pub(super) fn ask(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.active() else {
        return;
    };
    let (path, at) = (open.path.clone(), open.new.caret());
    let Some((dir, clients)) = clients_of(state, &path) else {
        state.workspace.servers.dismiss();
        return;
    };
    let asked = state.workspace.servers.offering(&path, at);
    let file = dir.join(&path);
    spawner.spawn(Box::pin(async move {
        let mut items = Vec::new();
        for client in clients {
            items.extend(client.completion(&file, at).await.unwrap_or_default());
        }
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.servers.offers(asked, items);
        }) as Continuation
    }));
}
