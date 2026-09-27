//! `git status --porcelain=v2`, as the files it names.

/// One `git status --porcelain` row: index letter, worktree letter, path, a rename on its new side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub x: char,
    pub y: char,
    pub path: String,
}

impl Change {
    pub fn is_untracked(&self) -> bool {
        self.x == '?' && self.y == '?'
    }

    pub fn is_staged(&self) -> bool {
        self.x != ' ' && self.x != '?'
    }

    pub fn is_modified(&self) -> bool {
        self.y != ' ' || self.is_untracked()
    }
}

/// A row too short to carry a path yields an empty `path`.
pub fn porcelain(text: &str) -> Vec<Change> {
    text.lines()
        .filter(|line| line.len() >= 2)
        .map(|line| {
            let mut letters = line.chars();
            let x = letters.next().unwrap_or(' ');
            let y = letters.next().unwrap_or(' ');
            let path = line
                .get(3..)
                .map(|rest| rest.rsplit(" -> ").next().unwrap_or(rest).to_string())
                .unwrap_or_default();
            Change { x, y, path }
        })
        .collect()
}
