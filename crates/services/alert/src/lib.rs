//! The alerts capability: the firing alerts as last read, and when to read them again.

use std::sync::{Arc, Mutex};

use groove_types::{Alert, AlertsConfig, Result, Timestamp};

#[cfg(test)]
mod tests;

/// The `alert` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// The firing alerts, the most urgent first.
    pub firing: Vec<Alert>,
    /// When the last read came back.
    pub read_at: Option<Timestamp>,
    /// When the read in flight was sent.
    pub sent_at: Option<Timestamp>,
    /// Why the last read failed, until one succeeds.
    pub failed: Option<String>,
}

impl State {
    /// Whether a read is due: none in flight, and none back for `every` seconds.
    pub fn due(&self, now: Timestamp, every: i64) -> bool {
        let fresh = |at: Timestamp| now.seconds() - at.seconds() < every;
        self.sent_at.is_none() && !self.read_at.is_some_and(fresh)
    }

    pub fn sent(&mut self, now: Timestamp) {
        self.sent_at = Some(now);
    }

    /// What a read answered, at `now`.
    pub fn read(&mut self, answered: Result<Vec<Alert>>, now: Timestamp) {
        self.sent_at = None;
        self.read_at = Some(now);
        match answered {
            Ok(firing) => {
                self.firing = firing;
                self.failed = None;
            }
            Err(e) => self.failed = Some(e.message),
        }
    }

    /// The firing alert of that key.
    pub fn find(&self, dedup_key: &str) -> Option<&Alert> {
        self.firing.iter().find(|one| one.dedup_key == dedup_key)
    }

    /// How many fire at each rank: critical, on call, warning, the rest.
    pub fn counts(&self) -> [usize; 4] {
        let mut out = [0; 4];
        for one in &self.firing {
            out[usize::from(one.rank())] += 1;
        }
        out
    }
}

/// Where the alerts are read: incident.io, or a list held for tests.
#[derive(Clone)]
pub struct Service {
    source: Source,
}

#[derive(Clone)]
enum Source {
    Incident(Arc<groove_alerts::Incident>),
    Held(Arc<Mutex<Vec<Alert>>>),
}

impl Service {
    /// incident.io, with the key `inc` holds.
    pub fn incident_io() -> Result<Self> {
        let incident = groove_alerts::Incident::logged_in()?;
        Ok(Self {
            source: Source::Incident(Arc::new(incident)),
        })
    }

    /// Reads that answer `alerts`, which `hold` changes; nothing is asked of incident.io.
    pub fn held(alerts: Vec<Alert>) -> Self {
        Self {
            source: Source::Held(Arc::new(Mutex::new(alerts))),
        }
    }

    /// What the held reads answer from now on.
    pub fn hold(&self, alerts: Vec<Alert>) {
        if let Source::Held(held) = &self.source {
            *held.lock().unwrap_or_else(|e| e.into_inner()) = alerts;
        }
    }

    /// The firing alerts of the config's teams, read off the main thread.
    pub async fn read(&self, config: &AlertsConfig) -> Result<Vec<Alert>> {
        match &self.source {
            Source::Incident(incident) => incident.firing(config).await,
            Source::Held(held) => Ok(held.lock().unwrap_or_else(|e| e.into_inner()).clone()),
        }
    }
}
