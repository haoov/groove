//! The worktree's paths, walked off the main thread.

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
        let read = groove_workspace_service::paths(&dir).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.workspace.walking = false;
                let selected = state.session.selected_worktree().map(|one| &one.id);
                match read {
                    Ok(_) if selected.is_none() => {}
                    Ok(_) if selected != Some(&id) => list_paths(state, spawner),
                    Ok(paths) => state.workspace.set_paths(paths),
                    Err(e) => state.failed(e),
                }
            },
        ) as Continuation
    }));
}
