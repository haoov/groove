//! What the language servers offer to type at the caret, asked again as the user types.

use groove_types::Edit;
use groove_workspace_service::Offering;

use super::servers::clients_of;
use crate::{AppState, Continuation, Services, Spawner};

/// What an edit does to the offer at the caret.
pub(super) fn after(state: &mut AppState, spawner: &dyn Spawner, edit: &Edit) {
    match state.workspace.servers.after(edit) {
        Offering::Ask => ask(state, spawner),
        Offering::Dismiss => state.workspace.servers.dismiss(),
        Offering::Keep => {}
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
