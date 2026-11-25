//! Argo CD read from its own API, on the token the argocd CLI keeps after its SSO login.

mod drift;
mod login;
mod revision;
mod tree;
mod windows;

#[cfg(test)]
mod tests;

pub use drift::{Drifted, drift, drift_path};
pub use login::{REFRESH, Token, config_path, token_of};
pub use revision::{revision, revision_path};
pub use tree::{SERVER_LABEL, health, streamed, tree_path};
pub use windows::{windows, windows_path};
