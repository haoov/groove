//! The cluster part of the app: the kubeconfig contexts found, and whether each one signs in.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use groove_types::{KubeContext, Login, Result};

pub use groove_contexts::paths;

/// The `cluster` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// What the last scan found; `None` before the first one ends.
    pub found: Option<Vec<KubeContext>>,
    pub scanning: bool,
    logins: Vec<(String, Login)>,
    checking: Vec<String>,
}

impl State {
    /// What the last check of `context` found.
    pub fn login(&self, context: &str) -> Option<&Login> {
        let held = self.logins.iter().find(|(name, _)| name == context);
        held.map(|(_, login)| login)
    }

    pub fn checking(&self, context: &str) -> bool {
        self.checking.iter().any(|name| name == context)
    }

    /// Marks a scan begun. False while one already runs.
    pub fn begin_scan(&mut self) -> bool {
        !std::mem::replace(&mut self.scanning, true)
    }

    /// Marks a check of `context` begun. False while one already runs.
    pub fn begin_check(&mut self, context: &str) -> bool {
        if self.checking(context) {
            return false;
        }
        self.checking.push(context.to_string());
        true
    }
}

#[derive(Debug)]
pub enum Event {
    Found(Vec<KubeContext>),
    /// The scan could not read the files; what it found before stays.
    Unread,
    Checked {
        context: String,
        login: Login,
    },
}

pub fn apply(state: &mut State, event: Event) {
    match event {
        Event::Found(contexts) => {
            state.found = Some(contexts);
            state.scanning = false;
        }
        Event::Unread => state.scanning = false,
        Event::Checked { context, login } => {
            state.checking.retain(|name| *name != context);
            state.logins.retain(|(name, _)| *name != context);
            state.logins.push((context, login));
        }
    }
}

/// Every context the files name.
pub async fn scan(paths: Vec<PathBuf>) -> Result<Vec<KubeContext>> {
    groove_contexts::found(&paths).await
}

/// Whether `context` signs in now.
pub async fn check(paths: Vec<PathBuf>, context: String) -> Login {
    groove_contexts::check(&paths, &context).await
}
