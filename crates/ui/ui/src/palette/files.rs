//! The palette after `#`: the selected worktree's files, the best fuzzy match first.

use groove_controllers::AppState;

use super::{Act, Entry, Run};

/// The most files listed at once.
const SHOWN: usize = 200;

pub(super) fn rows(app: &AppState, query: &str) -> Vec<Entry> {
    let query: String = query.split_whitespace().collect();
    let query = query.as_str();
    let mut found: Vec<(i32, usize, &str)> = app
        .workspace
        .paths()
        .iter()
        .map(|one| one.path.as_str())
        .filter(|path| groove_types::fuzzy(path, query))
        .filter_map(|path| Some((-groove_types::sought(path, query)?.score, path.len(), path)))
        .collect();
    found.sort();
    let found = found.into_iter().take(SHOWN);
    found
        .map(|(_, _, path)| Entry {
            group: "Files",
            label: path.to_string(),
            run: Run::Act(Act::Open(path.to_string())),
        })
        .collect()
}
