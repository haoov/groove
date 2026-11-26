//! The changed files as the modes sidebar lists them: a tree of foldable directories, or flat.

use std::collections::{BTreeMap, BTreeSet};

use groove_types::FileDiff;

/// One row of the list: a directory, or a changed file.
pub(crate) struct Line<'a> {
    pub depth: usize,
    /// A directory's whole path, the key that folds it; a file's name.
    pub path: String,
    pub name: String,
    pub kind: Kind<'a>,
}

pub(crate) enum Kind<'a> {
    /// Whether its files stand under it, and how many it holds in all.
    Dir(bool, usize),
    /// The file, and its folder dimmed after its name in the flat list.
    File(&'a FileDiff, Option<String>),
}

#[derive(Default)]
struct Node<'a> {
    dirs: BTreeMap<String, Node<'a>>,
    files: Vec<&'a FileDiff>,
}

impl Node<'_> {
    fn count(&self) -> usize {
        self.files.len() + self.dirs.values().map(Node::count).sum::<usize>()
    }
}

/// The rows the list shows: flat by path, or the tree with the `closed` directories folded.
pub(crate) fn lines<'a>(
    files: &[&'a FileDiff],
    flat: bool,
    closed: &BTreeSet<String>,
) -> Vec<Line<'a>> {
    if flat {
        return sorted(files).into_iter().map(flat_line).collect();
    }
    let mut out = Vec::new();
    walk(&built(files), "", 0, closed, &mut out);
    out
}

/// Every changed file in the order the list shows it, folded or not.
pub(crate) fn order<'a>(files: &[&'a FileDiff], flat: bool) -> Vec<&'a FileDiff> {
    let lines = lines(files, flat, &BTreeSet::new());
    let file = |one: Line<'a>| match one.kind {
        Kind::File(file, _) => Some(file),
        Kind::Dir(..) => None,
    };
    lines.into_iter().filter_map(file).collect()
}

fn sorted<'a>(files: &[&'a FileDiff]) -> Vec<&'a FileDiff> {
    let mut out = files.to_vec();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn flat_line(file: &FileDiff) -> Line<'_> {
    let name = crate::views::name_of(&file.path).to_string();
    let dir = file.path.strip_suffix(name.as_str()).unwrap_or_default();
    let dir = dir.trim_end_matches('/');
    Line {
        depth: 0,
        path: file.path.clone(),
        name,
        kind: Kind::File(file, (!dir.is_empty()).then(|| dir.to_string())),
    }
}

fn built<'a>(files: &[&'a FileDiff]) -> Node<'a> {
    let mut root = Node::default();
    for file in sorted(files) {
        let mut parts: Vec<&str> = file.path.split('/').collect();
        parts.pop();
        let mut node = &mut root;
        for part in parts {
            node = node.dirs.entry(part.to_string()).or_default();
        }
        node.files.push(file);
    }
    root
}

/// One directory's rows: each directory under it, a chain of lone directories read as one,
/// then its files.
fn walk<'a>(
    node: &Node<'a>,
    at: &str,
    depth: usize,
    closed: &BTreeSet<String>,
    out: &mut Vec<Line<'a>>,
) {
    for (name, mut dir) in &node.dirs {
        let mut name = name.clone();
        while dir.files.is_empty() && dir.dirs.len() == 1 {
            let Some((next, inner)) = dir.dirs.iter().next() else {
                break;
            };
            name = format!("{name}/{next}");
            dir = inner;
        }
        let path = match at.is_empty() {
            true => name.clone(),
            false => format!("{at}/{name}"),
        };
        let open = !closed.contains(&path);
        out.push(Line {
            depth,
            path: path.clone(),
            name,
            kind: Kind::Dir(open, dir.count()),
        });
        if open {
            walk(dir, &path, depth + 1, closed, out);
        }
    }
    for file in &node.files {
        let name = crate::views::name_of(&file.path).to_string();
        out.push(Line {
            depth,
            path: file.path.clone(),
            name,
            kind: Kind::File(file, None),
        });
    }
}
