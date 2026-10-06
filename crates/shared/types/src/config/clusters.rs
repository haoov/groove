//! The kubeconfig contexts Groove knows, each with a configuration of its own.

/// One context added to Groove. The credentials stay in the kubeconfig.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClusterConfig {
    pub context: String,
    #[serde(default)]
    pub hue: Hue,
    /// No write at all, by the user or the agent.
    #[serde(default)]
    pub read_only: bool,
    /// The Argo CD hub, whose Applications fan out to the other clusters.
    #[serde(default)]
    pub argo_hub: bool,
}

impl ClusterConfig {
    /// A context just added, in the first hue no other context holds.
    pub fn new(context: &str, taken: &[Hue]) -> Self {
        let hue = Hue::ALL
            .into_iter()
            .find(|one| !taken.contains(one))
            .unwrap_or_default();
        Self {
            context: context.to_string(),
            hue,
            read_only: false,
            argo_hub: false,
        }
    }

    pub fn change(&mut self, change: ClusterChange) {
        match change {
            ClusterChange::ReadOnly(on) => self.read_only = on,
            ClusterChange::ArgoHub(on) => self.argo_hub = on,
        }
    }
}

/// One setting of a context changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClusterChange {
    ReadOnly(bool),
    ArgoHub(bool),
}

/// The colour a context is drawn in, wherever it appears.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hue {
    #[default]
    Sapphire,
    Mauve,
    Pink,
    Teal,
    Flamingo,
    Sky,
}

impl Hue {
    pub const ALL: [Hue; 6] = [
        Hue::Sapphire,
        Hue::Mauve,
        Hue::Pink,
        Hue::Teal,
        Hue::Flamingo,
        Hue::Sky,
    ];
}
