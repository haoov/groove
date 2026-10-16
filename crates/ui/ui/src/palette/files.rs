//! The palette after `#`: the selected worktree's files, the best fuzzy match first.

use std::cell::RefCell;
use std::rc::Rc;

use groove_controllers::AppState;

use super::{Act, Entry, Run};

/// The most files listed at once.
const SHOWN: usize = 200;

/// The paths a query last found, and the query and paths' stamp they were found for.
#[derive(Debug, Clone, Default)]
pub struct Found(RefCell<Option<(Asked, Rc<Vec<String>>)>>);

/// The query, and the stamp of the paths it ran over.
type Asked = (String, u64);

/// A cache: two palettes are the same whatever each last found.
impl PartialEq for Found {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Found {}

pub(super) fn rows(app: &AppState, query: &str, found: &Found) -> Vec<Entry> {
    let query: String = query.split_whitespace().collect();
    let key = (query.clone(), app.workspace.paths_stamp());
    let mut kept = found.0.borrow_mut();
    let paths = match kept.as_ref().filter(|(at, _)| *at == key) {
        Some((_, paths)) => paths.clone(),
        None => {
            let paths = Rc::new(best(app, &query));
            *kept = Some((key, paths.clone()));
            paths
        }
    };
    let entry = |path: &String| Entry {
        group: "Files",
        label: path.clone(),
        run: Run::Act(Act::Open(path.clone())),
    };
    paths.iter().map(entry).collect()
}

/// The paths the query finds, the best first, as many as are shown.
fn best(app: &AppState, query: &str) -> Vec<String> {
    let mut found: Vec<(i32, usize, &str)> = app
        .workspace
        .paths()
        .iter()
        .map(|one| one.path.as_str())
        .filter(|path| groove_types::fuzzy(path, query))
        .filter_map(|path| Some((-groove_types::sought(path, query)?.score, path.len(), path)))
        .collect();
    found.sort();
    found
        .into_iter()
        .take(SHOWN)
        .map(|(_, _, path)| path.to_string())
        .collect()
}
