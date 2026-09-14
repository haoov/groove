use std::path::PathBuf;

use groove_types::ErrorKind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Exec(#[from] groove_exec::Error),
    #[error("{command} failed: {stderr}")]
    Failed { command: String, stderr: String },
    #[error("{command} printed something unexpected: {output}")]
    Unexpected { command: String, output: String },
    #[error("no base branch on origin in {}: tried {}", dir.display(), tried.join(", "))]
    NoBase { dir: PathBuf, tried: Vec<String> },
    #[error("{} already exists", path.display())]
    AlreadyExists { path: PathBuf },
    #[error("{branch} has diverged from origin; pull cannot fast-forward")]
    Diverged { branch: String },
    #[error("cannot parse git URL: {0}")]
    BadUrl(String),
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error::AlreadyExists { .. } | Error::Diverged { .. } => ErrorKind::Conflict,
            Error::BadUrl(_) => ErrorKind::Invalid,
            _ => ErrorKind::Git,
        };
        groove_types::Error::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
