//! What the session's middle shows, picked from the bar's tabs.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Overview,
    #[default]
    Agent,
    Diff,
    Files,
    Kubernetes,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::Overview,
        Mode::Agent,
        Mode::Diff,
        Mode::Files,
        Mode::Kubernetes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Overview => "Overview",
            Mode::Agent => "Agent",
            Mode::Diff => "Diff",
            Mode::Files => "Files",
            Mode::Kubernetes => "Kubernetes",
        }
    }
}
