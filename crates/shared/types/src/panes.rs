//! Where the user left the window's boundaries, as the file on disk holds them.

/// Every column's width but the workspace's, and the commit box's height, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Panes {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    pub commit: f32,
    /// How tall the board's timeline stands.
    #[serde(default = "band")]
    pub band: f32,
    /// How tall the rail's feed stands, its footer included.
    #[serde(default = "feed")]
    pub feed: f32,
    /// How tall the manual section stands open.
    #[serde(default = "manual")]
    pub manual: f32,
}

fn manual() -> f32 {
    220.0
}

fn feed() -> f32 {
    200.0
}

fn band() -> f32 {
    260.0
}
