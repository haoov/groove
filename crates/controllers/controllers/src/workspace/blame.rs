//! Who last changed each line of a file, read once per read of it.

use crate::{AppState, Continuation, Services, Spawner};

/// The blame of `path` in the selected worktree, unless this read of it already has one.
pub(super) fn blame(state: &mut AppState, spawner: &dyn Spawner, path: String) {
    let Some(asked) = super::Asked::now(state).filter(|_| !state.workspace.readonly()) else {
        return;
    };
    let read = state.workspace.read_of(&path);
    if !state.workspace.blames.owed(&asked.worktree, &path, read) {
        return;
    }
    state.workspace.blames.asked(&asked.worktree, &path, read);
    let contents = state.workspace.buffer(&path).map(|open| open.new.text());
    let job = state.begin(format!("blaming {path}"));
    spawner.spawn(Box::pin(async move {
        let lines = groove_workspace_service::blame(&asked.dir, &path, contents).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            let blames = &mut state.workspace.blames;
            match lines {
                Ok(lines) => blames.took(&asked.worktree, path, read, lines),
                Err(e) => {
                    blames.failed(&path, read);
                    state.failed(e);
                }
            }
        }) as Continuation
    }));
}
