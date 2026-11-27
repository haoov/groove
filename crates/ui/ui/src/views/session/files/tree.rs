//! The changed files as paths: their shared root, the groups under it, each file's name.

/// The name a file reads as, and its path; `mod.rs` and its like read as their directory.
pub(crate) fn reads_as(path: &str) -> (String, String) {
    let name = crate::views::name_of(path);
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
