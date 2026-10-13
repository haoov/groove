//! The config and the panes written off the main thread, one write at a time, the newest last.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::{Config, Error, Panes};

/// What waits for the disk, and the config revision last on it.
#[derive(Debug, Clone, Default)]
pub struct Keeper {
    held: Arc<Mutex<Held>>,
    writing: Arc<tokio::sync::Mutex<()>>,
    kept: Arc<AtomicU64>,
}

#[derive(Debug, Default)]
struct Held {
    config: Option<(PathBuf, Config, u64)>,
    panes: Option<(PathBuf, Panes)>,
    /// The highest config revision handed to a write, written or not.
    tried: u64,
}

impl Keeper {
    /// `config` at `revision` held for the next flush, unless a later one is held or was tried.
    pub fn hold(&self, config_dir: &Path, config: Config, revision: u64) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let newest = held.config.as_ref().map_or(held.tried, |(_, _, at)| *at);
        if revision > newest {
            held.config = Some((config_dir.to_path_buf(), config, revision));
        }
    }

    /// `panes` held for the next flush, in place of any held before.
    pub fn hold_panes(&self, data_dir: &Path, panes: Panes) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.panes = Some((data_dir.to_path_buf(), panes));
    }

    /// The config revision last written to disk.
    pub fn kept(&self) -> u64 {
        self.kept.load(Ordering::SeqCst)
    }

    /// A config that failed to write, held again for the next flush unless a newer one is held.
    fn held_back(&self, failed: (PathBuf, Config, u64)) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.config.as_ref().is_none_or(|(_, _, at)| *at < failed.2) {
            held.config = Some(failed);
        }
    }

    /// The last flush, at quit: an error too when a config tried before never reached the disk.
    pub async fn quit(&self) -> Result<(), Error> {
        self.flush().await?;
        let tried = self
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tried;
        match tried > self.kept() {
            true => Err(Error::new(
                groove_types::ErrorKind::Io,
                format!("config revision {tried} never reached the disk"),
            )),
            false => Ok(()),
        }
    }

    /// Everything held, written; the first failure of this flush.
    pub async fn flush(&self) -> Result<(), Error> {
        let _writing = self.writing.lock().await;
        let (config, panes) = {
            let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, _, revision)) = &held.config {
                held.tried = *revision;
            }
            (held.config.take(), held.panes.take())
        };
        let mut flushed = Ok(());
        if let Some((dir, config, revision)) = config {
            match groove_config::write(&groove_config::path(&dir), &config).await {
                Ok(()) => _ = self.kept.fetch_max(revision, Ordering::SeqCst),
                Err(e) => {
                    self.held_back((dir, config, revision));
                    flushed = Err(e.into());
                }
            }
        }
        if let Some((dir, panes)) = panes {
            let path = groove_config::panes::path(&dir);
            let written = groove_config::panes::write(&path, &panes).await;
            flushed = flushed.and(written.map_err(Error::from));
        }
        flushed
    }
}
