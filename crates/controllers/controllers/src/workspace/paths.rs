//! The worktree's own files: one made, moved, copied or taken away.

use groove_workspace_service::{PathOp, path_op};

use crate::{AppState, Continuation, Services, Spawner};

/// One path operation, behind the worktree's file operations out; then the worktree read again.
pub(super) fn act(state: &mut AppState, spawner: &dyn Spawner, op: PathOp) {
    let Some((id, dir)) = state
        .session
        .selected_worktree()
        .map(|one| (one.id.clone(), one.dir()))
    else {
        return;
    };
    let job = state.begin(label(&op));
    let gone = match &op {
        PathOp::Delete { path } => Some(path.clone()),
        _ => None,
    };
    let mut turn = state.workspace.filing(&id);
    spawner.spawn(Box::pin(async move {
        turn.wait().await;
        let done = path_op(&dir, &op).await;
        drop(turn);
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if let Err(e) = done {
                    return state.failed(e);
                }
                state.workspace.clear_paths();
                if let Some(path) = gone.as_ref() {
                    state.workspace.shut_if_gone(path);
                }
                super::diff::reread(state, spawner);
            },
        ) as Continuation
    }));
}

fn label(op: &PathOp) -> String {
    match op {
        PathOp::Create { path, .. } => format!("making {path}"),
        PathOp::Rename { from, to } => format!("renaming {from} to {to}"),
        PathOp::Copy { from, to } => format!("copying {from} to {to}"),
        PathOp::Delete { path } => format!("deleting {path}"),
    }
}
