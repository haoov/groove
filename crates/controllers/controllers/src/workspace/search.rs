//! Text across the worktree, walked off the main thread and reported in batches.

use groove_workspace_service::Search;

use super::worktree_dir;
use crate::{AppState, Continuation, Services, Spawner};

/// Every line of the worktree holding `query`, walked on a thread of its own and
/// reported in batches. A search still running gives up for this one.
pub(super) fn grep(state: &mut AppState, spawner: &dyn Spawner, query: String, under: String) {
    state.workspace.stop();
    state.workspace.found.clear();
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    if query.is_empty() {
        return;
    }
    let search = std::sync::Arc::new(Search::default());
    state.workspace.searching = Some(search.clone());
    let sink = spawner.sink();
    spawner.spawn(Box::pin(async move {
        let reading = tokio::task::spawn_blocking(move || {
            groove_workspace_service::grep(&dir, &query, &under, search, |batch| {
                sink.deliver(Box::new(
                    move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                        state.workspace.found.extend(batch);
                    },
                ));
            });
        });
        let _ = reading.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.searching = None;
        }) as Continuation
    }));
}
