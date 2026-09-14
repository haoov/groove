use std::path::PathBuf;

use groove_types::ErrorKind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Git(#[from] groove_git::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("no {what} {id}")]
    NotFound { what: &'static str, id: String },
    #[error("invalid branch name '{branch}': {reason}")]
    InvalidBranch { branch: String, reason: String },
    #[error("'{0}' is not a <host>/<group>/<project> pool path")]
    BadSlug(String),
    #[error("{} has no origin remote", path.display())]
    NoOrigin { path: PathBuf },
    #[error("no repo named {0} in the pool")]
    UnknownRepo(String),
    #[error("{name} matches several repos: {}", matches.join(", "))]
    AmbiguousRepo { name: String, matches: Vec<String> },
    #[error("{} already exists", path.display())]
    Exists { path: PathBuf },
    #[error("no branch {target} on origin; it has: {}", available.join(", "))]
    NoTarget {
        target: String,
        available: Vec<String>,
    },
    #[error(
        "{repo} already has a local branch {branch}, and this session derived that name rather than being given it. \
         Name the branch to continue it on purpose, or rename this session or that branch to start fresh."
    )]
    ForeignBranch { repo: String, branch: String },
    #[error("the worktree has uncommitted changes; commit or discard first, or force")]
    Dirty,
    #[error("{branch} has {ahead} commits origin does not; push first, or force")]
    Unpushed { branch: String, ahead: u32 },
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        let kind = match &e {
            Error::Git(_) => {
                let Error::Git(g) = e else { unreachable!() };
                return g.into();
            }
            Error::Sqlx(_) => ErrorKind::Db,
            Error::Io { .. } => ErrorKind::Io,
            Error::NotFound { .. } | Error::UnknownRepo(_) => ErrorKind::NotFound,
            Error::InvalidBranch { .. }
            | Error::BadSlug(_)
            | Error::AmbiguousRepo { .. }
            | Error::NoTarget { .. } => ErrorKind::Invalid,
            Error::NoOrigin { .. }
            | Error::Exists { .. }
            | Error::ForeignBranch { .. }
            | Error::Dirty
            | Error::Unpushed { .. } => ErrorKind::Conflict,
        };
        groove_types::Error::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
