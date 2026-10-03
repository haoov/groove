//! The surfaces, one directory each: a view reads `AppState` and composes widgets.

pub mod board;
pub mod overlays;
pub mod rail;
pub mod session;
pub mod settings;
pub mod splitter;

/// The last part of a path: the file or directory it names.
pub(crate) fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
