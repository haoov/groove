//! How the changed files read as paths: the root they share, the groups under it, and
//! the name a file goes by.

use groove_types::FileDiff;

pub(crate) struct Listing<'a> {
    /// What every file has in common, shown once above them.
    pub root: String,
    pub groups: Vec<Group<'a>>,
}

pub(crate) struct Group<'a> {
    /// The directory under the root, its single-child chains collapsed.
    pub dir: String,
    pub files: Vec<&'a FileDiff>,
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub(crate) fn listing<'a>(files: &[&'a FileDiff]) -> Listing<'a> {
    let root = common(files);
    let mut groups: Vec<Group<'a>> = Vec::new();
    for file in files {
        let dir = under(&root, &file.path);
        match groups.iter_mut().find(|group| group.dir == dir) {
            Some(group) => group.files.push(file),
            None => groups.push(Group {
                dir,
                files: vec![file],
            }),
        }
    }
    groups.sort_by(|a, b| a.dir.cmp(&b.dir));
    Listing { root, groups }
}

/// The longest path every file shares, and nothing when they share none.
fn common(files: &[&FileDiff]) -> String {
    let mut shared: Option<Vec<&str>> = None;
    for file in files {
        let parts = segments(&file.path);
        shared = Some(match shared {
            None => parts,
            Some(shared) => shared
                .iter()
                .zip(parts.iter())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| *a)
                .collect(),
        });
    }
    shared.unwrap_or_default().join("/")
}

/// A file's directory with the root taken off its front.
fn under(root: &str, path: &str) -> String {
    let dir = segments(path).join("/");
    match dir.strip_prefix(root) {
        Some(rest) => rest.trim_start_matches('/').to_string(),
        None => dir,
    }
}

/// The name a file reads as, and the path behind it. `mod.rs` and its like read as
/// the directory that holds them.
pub(crate) fn reads_as(path: &str) -> (String, String) {
    let name = name_of(path);
    let parts = segments(path);
    let plain = [
        "mod.rs", "lib.rs", "main.rs", "index.ts", "index.js", "mod.ts",
    ];
    match plain.contains(&name) {
        true => match parts.last() {
            Some(dir) => (format!("{dir}/{name}"), parts[..parts.len() - 1].join("/")),
            None => (name.to_string(), String::new()),
        },
        false => (name.to_string(), parts.join("/")),
    }
}

/// A path's directories, without its file name.
fn segments(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts
}
