//! Where the symbol at a spot of the open file is defined: opened there, or said at the spot.

use std::path::Path;

use groove_types::{Caret, Location, Selection};

use groove_workspace_service::{Landing, landing, none_up};

use super::editor::open_file;
use super::servers::clients_of;
use crate::{AppState, Continuation, Services, Spawner};

/// Asks the servers of `path`; the first place inside the worktree opens with the caret on it.
pub(super) fn definition(state: &mut AppState, spawner: &dyn Spawner, path: String, at: Caret) {
    let Some((dir, clients)) = clients_of(state, &path) else {
        return state.workspace.servers.told(&path, at, none_up(&path));
    };
    let worktree = state.workspace.worktree.clone();
    let file = dir.join(&path);
    spawner.spawn(Box::pin(async move {
        let mut found = Vec::new();
        for client in clients {
            if let Ok(places) = client.definition(&file, at).await
                && !places.is_empty()
            {
                found = places;
                break;
            }
        }
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if state.workspace.worktree == worktree {
                    landed(state, spawner, (&path, at), &dir, found);
                }
            },
        ) as Continuation
    }));
}

fn landed(
    state: &mut AppState,
    spawner: &dyn Spawner,
    (from, at): (&str, Caret),
    dir: &Path,
    found: Vec<Location>,
) {
    match landing(dir, &found) {
        Landing::Open(path, start) => {
            state.workspace.servers.landed(&path, start);
            open_file(state, spawner, path, Some(Selection::at(start)));
        }
        Landing::Said(said) => state.workspace.servers.told(from, at, said),
    }
}
