//! Where the user left the window's boundaries, as the file on disk holds them.

/// Every column's width but the workspace's, in logical pixels, and how tall the
/// commit box stands.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Panes {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    pub commit: f32,
    /// How tall the board's timeline stands.
    #[serde(default = "band")]
    pub band: f32,
}

fn band() -> f32 {
    260.0
}
