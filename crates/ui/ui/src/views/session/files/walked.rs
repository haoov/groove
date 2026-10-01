//! The worktree's paths as directories, built again only when the paths move.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use groove_types::FileDiff;

/// What every directory holds, by its own path from the worktree root.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub dirs: BTreeSet<String>,
    pub files: BTreeSet<String>,
}

pub(crate) type Tree = BTreeMap<String, Node>;

/// The last tree built, and the paths stamp it was built for.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Walked {
    kept: RefCell<Option<(u64, Rc<Tree>)>>,
}

impl Walked {
    pub(crate) fn of(&self, paths: &[FileDiff], stamp: u64) -> Rc<Tree> {
        let mut kept = self.kept.borrow_mut();
        if let Some((at, tree)) = kept.as_ref()
            && *at == stamp
        {
            return tree.clone();
        }
        let tree = Rc::new(tree(paths));
        *kept = Some((stamp, tree.clone()));
        tree
    }
}

fn tree(paths: &[FileDiff]) -> Tree {
    let mut tree = Tree::new();
    for file in paths {
        let mut parent = String::new();
        let parts: Vec<&str> = file.path.split('/').collect();
        for (at, part) in parts.iter().enumerate() {
            let last = at + 1 == parts.len();
            let here = match parent.is_empty() {
                true => (*part).to_string(),
                false => format!("{parent}/{part}"),
            };
            let node = tree.entry(parent.clone()).or_default();
            match last {
                true => node.files.insert(here.clone()),
                false => node.dirs.insert(here.clone()),
            };
            parent = here;
        }
    }
    tree
}
