use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        groove_types::Error::new(groove_types::ErrorKind::Agent, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
