//! Where the user left the window's boundaries, as the file on disk holds them.

/// Every column's width but the workspace's, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Panes {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    /// How tall the rail's feed stands, its footer included.
    #[serde(default = "feed")]
    pub feed: f32,
    /// How tall the manual section stands open.
    #[serde(default = "manual")]
    pub manual: f32,
    /// How far down a maintenance task's alerts reach open.
    #[serde(default = "alerts")]
    pub alerts: f32,
}

fn alerts() -> f32 {
    280.0
}

fn manual() -> f32 {
    220.0
}

fn feed() -> f32 {
    200.0
}
