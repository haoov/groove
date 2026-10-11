//! Text across the worktree, walked off the main thread and reported in batches.

use groove_workspace_service::Search;

use crate::{AppState, Continuation, Services, Spawner};

/// Every file of the selected worktree, for the path term to narrow by.
pub(super) fn list_paths(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = state.session.selected_worktree() else {
        return;
    };
    let (id, dir) = (worktree.id.clone(), worktree.dir());
    if state.workspace.walking {
        return;
    }
    state.workspace.walking = true;
    spawner.spawn(Box::pin(async move {
        let read = tokio::task::spawn_blocking(move || groove_workspace_service::paths(&dir)).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.workspace.walking = false;
                let selected = state.session.selected_worktree().map(|one| &one.id);
                match read {
                    Ok(_) if selected.is_none() => {}
                    Ok(_) if selected != Some(&id) => list_paths(state, spawner),
                    Ok(paths) => state.workspace.set_paths(paths),
                    Err(e) => state.failed(groove_types::Error::internal(format!(
                        "the walk failed: {e}"
                    ))),
                }
            },
        ) as Continuation
    }));
}

/// Every line of the worktree holding `query`, walked on a thread and reported in batches.
pub(super) fn grep(state: &mut AppState, spawner: &dyn Spawner, query: String, under: String) {
    state.workspace.stop();
    state.workspace.found.clear();
    let Some(dir) = state
        .session
        .selected_worktree()
        .map(groove_types::Worktree::dir)
    else {
        return;
    };
    if query.is_empty() {
        return;
    }
    let search = std::sync::Arc::new(Search::default());
    state.workspace.searching = Some(search.clone());
    let (sink, mine) = (spawner.sink(), search.clone());
    let current = move |state: &AppState, one: &std::sync::Arc<Search>| {
        let held = state.workspace.searching.as_ref();
        held.is_some_and(|held| std::sync::Arc::ptr_eq(held, one))
    };
    spawner.spawn(Box::pin(async move {
        let walked = search.clone();
        let reading = tokio::task::spawn_blocking(move || {
            groove_workspace_service::grep(&dir, &query, &under, walked.clone(), |batch| {
                let walked = walked.clone();
                sink.deliver(Box::new(
                    move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                        if current(state, &walked) {
                            state.workspace.found.extend(batch);
                        }
                    },
                ));
            });
        });
        let _ = reading.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if current(state, &mine) {
                state.workspace.searching = None;
            }
        }) as Continuation
    }));
}
