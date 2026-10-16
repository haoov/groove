//! Where the symbol at a spot of the open file is defined: opened there when the worktree
//! holds it, said at the spot when it does not.

use std::path::Path;

use groove_types::{Caret, Location, Selection};

use super::editor::open_file;
use super::servers::clients_of;
use crate::{AppState, Continuation, Services, Spawner};

/// Asks the servers of `path`; the first place inside the worktree opens with the caret on it.
pub(super) fn definition(state: &mut AppState, spawner: &dyn Spawner, path: String, at: Caret) {
    let Some((dir, clients)) = clients_of(state, &path) else {
        let said = format!("No language server is up for {path}");
        return state.workspace.servers.told(&path, at, said);
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
    let Some(first) = found.first() else {
        let said = "No definition here".to_string();
        return state.workspace.servers.told(from, at, said);
    };
    match first.path.strip_prefix(dir) {
        Ok(inside) => {
            let path = inside.to_string_lossy().into_owned();
            state.workspace.servers.landed(&path, first.start);
            open_file(state, spawner, path, Some(Selection::at(first.start)));
        }
        Err(_) => {
            let said = format!(
                "Defined outside the worktree, in {}. Groove opens worktree paths only.",
                first.path.display()
            );
            state.workspace.servers.told(from, at, said);
        }
    }
}
