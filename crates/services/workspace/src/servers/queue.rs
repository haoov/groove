//! What waits for a server to start: one text a file, however many changes came.

use super::Tell;

impl Tell {
    fn path(&self) -> &str {
        match self {
            Tell::Opened { path, .. } | Tell::Changed { path, .. } => path,
            Tell::Saved { path } | Tell::Closed { path } => path,
        }
    }
}

/// `told` queued; a change takes the text of the file's last queued open or change instead.
pub(super) fn queue(queued: &mut Vec<Tell>, told: Tell) {
    let Tell::Changed { path, text } = &told else {
        return queued.push(told);
    };
    let last = queued.iter_mut().rev().find(|one| one.path() == path);
    match last {
        Some(Tell::Opened { text: held, .. } | Tell::Changed { text: held, .. }) => {
            held.clone_from(text);
        }
        _ => queued.push(told),
    }
}
