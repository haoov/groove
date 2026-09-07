//! Transport for the GitLab and GitHub APIs: tokens, HTTP, error shapes.

pub(crate) mod api;
pub(crate) mod auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Gitlab,
    Github,
}
