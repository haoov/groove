use std::path::PathBuf;

use groove_types::ErrorKind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a valid config: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error::Read { .. } | Error::Write { .. } => ErrorKind::Io,
            Error::Parse { .. } => ErrorKind::Invalid,
        };
        groove_types::Error::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
