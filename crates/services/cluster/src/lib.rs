//! The cluster part of the app: the contexts found, their sign-in, and the watched objects.

mod store;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use groove_types::{FollowKey, KubeContext, KubeKind, Login, Result, Usage, WatchKey};

pub use groove_contexts::paths;
pub use groove_objects::{Batch, Change, Delta, Followed, Stop};
pub use store::{Follow, Follows, Named, Store, Watched, Yamls};

/// The `cluster` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// What the last scan found; `None` before the first one ends.
    pub found: Option<Vec<KubeContext>>,
    pub scanning: bool,
    logins: Vec<(String, Login)>,
    checking: Vec<String>,
    pub store: Store,
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
    Kinds {
        context: String,
        kinds: Vec<KubeKind>,
    },
    Watched {
        key: Box<WatchKey>,
        batch: Batch,
    },
    Followed {
        key: Box<FollowKey>,
        batch: Followed,
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
        Event::Kinds { context, kinds } => state.store.set_kinds(&context, kinds),
        Event::Watched { key, batch } => state.store.apply(&key, batch),
        Event::Followed { key, batch } => state.store.follows.apply(&key, batch),
    }
}

/// Every context the files name.
pub async fn scan(paths: Vec<PathBuf>) -> Result<Vec<KubeContext>> {
    groove_contexts::found(&paths).await
}

/// The kinds of `context`, from the cache under `cache` while it is fresh unless `again`.
pub async fn kinds(
    paths: Vec<PathBuf>,
    context: String,
    cache: PathBuf,
    again: bool,
) -> Result<Vec<KubeKind>> {
    groove_objects::kinds(&paths, &context, &cache, again).await
}

/// The names of every namespace of `context`.
pub async fn namespaces(paths: Vec<PathBuf>, context: String) -> Result<Vec<String>> {
    groove_objects::namespaces(&paths, &context).await
}

/// The watcher of `key`, until `stop`; each batch goes through `send`.
pub fn watcher(
    paths: Vec<PathBuf>,
    key: WatchKey,
    stop: &Stop,
    send: impl Fn(Batch) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    groove_objects::watch(paths, key, stop, send)
}

/// The whole-object watcher of `key`, until `stop`; each batch goes through `send`.
pub fn follower(
    paths: Vec<PathBuf>,
    key: FollowKey,
    stop: &Stop,
    send: impl Fn(Followed) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    groove_objects::follow(paths, key, stop, send)
}

/// What each container of a pod uses now; none where the cluster runs no metrics-server.
pub async fn usage(paths: Vec<PathBuf>, pod: Named) -> Result<Option<Usage>> {
    groove_objects::usage(&paths, &pod.0, &pod.1, &pod.2).await
}

/// The latest revision of a Helm release.
pub async fn helm_revision(paths: Vec<PathBuf>, release: Named) -> Result<Option<u32>> {
    groove_objects::helm_revision(&paths, &release.0, &release.1, &release.2).await
}

/// Whether `context` signs in now.
pub async fn check(paths: Vec<PathBuf>, context: String) -> Login {
    groove_contexts::check(&paths, &context).await
}
