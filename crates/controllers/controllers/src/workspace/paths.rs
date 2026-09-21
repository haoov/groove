//! The worktree's own files: one made, moved, copied or taken away.

use groove_workspace_service::{PathOp, path_op};

use super::worktree_dir;
use crate::{AppState, Continuation, Services, Spawner};

/// One path operation, then the worktree read again.
pub(super) fn act(state: &mut AppState, spawner: &dyn Spawner, op: PathOp) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(label(&op));
    let gone = match &op {
        PathOp::Delete { path } => Some(path.clone()),
        _ => None,
    };
    spawner.spawn(Box::pin(async move {
        let done = path_op(&dir, &op);
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if let Err(e) = done {
                    return state.errors.push(e);
                }
                state.workspace.paths.clear();
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
