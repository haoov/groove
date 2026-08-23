//! Transport for the GitLab and GitHub APIs: tokens, HTTP, error shapes.
//!
//! Lives in `core` because two features need it — `forge/` for MRs and
//! `provider/github/` for issues.

pub(crate) mod api;
pub(crate) mod auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Gitlab,
    Github,
}
