//! The config written off the main thread, the newest change last whatever order the writes end in.

use std::path::Path;
use std::sync::Arc;

use groove_types::{Config, Error};

/// The revision of the config last written.
#[derive(Debug, Clone, Default)]
pub struct Keeper {
    last: Arc<tokio::sync::Mutex<u64>>,
}

impl Keeper {
    /// `config` at `revision` written, unless a later revision is on disk already.
    pub async fn keep(
        &self,
        config_dir: &Path,
        config: &Config,
        revision: u64,
    ) -> Result<(), Error> {
        let mut last = self.last.lock().await;
        if revision <= *last {
            return Ok(());
        }
        groove_config::write(&groove_config::path(config_dir), config).await?;
        *last = revision;
        Ok(())
    }
}
