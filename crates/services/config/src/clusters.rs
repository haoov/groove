//! The kubeconfig contexts added to Groove, and their own configuration.

use groove_types::{ClusterChange, ClusterConfig, Config};

use crate::State;

impl State {
    pub fn clusters(&self) -> &[ClusterConfig] {
        self.config
            .as_ref()
            .map(|config| config.clusters.as_slice())
            .unwrap_or_default()
    }

    pub fn cluster(&self, context: &str) -> Option<&ClusterConfig> {
        self.clusters().iter().find(|one| one.context == context)
    }

    /// A context added once; adding it again changes nothing.
    pub fn add_cluster(&mut self, context: &str) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.clusters;
        if !held.iter().any(|one| one.context == context) {
            let taken: Vec<_> = held.iter().map(|one| one.hue).collect();
            held.push(ClusterConfig::new(context, &taken));
        }
        self.config.as_ref()
    }

    pub fn remove_cluster(&mut self, context: &str) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.clusters;
        held.retain(|one| one.context != context);
        self.config.as_ref()
    }

    /// One setting of an added context; a context not added changes nothing.
    pub fn change_cluster(&mut self, context: &str, change: ClusterChange) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.clusters;
        let one = held.iter_mut().find(|one| one.context == context)?;
        one.change(change);
        self.config.as_ref()
    }
}
