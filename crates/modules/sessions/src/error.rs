use groove_types::{ErrorKind, SessionId};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("no {what} {id}")]
    NotFound { what: &'static str, id: String },
    #[error("{id} is not a {expected} session")]
    WrongKind {
        expected: &'static str,
        id: SessionId,
    },
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error::NotFound { .. } => ErrorKind::NotFound,
            Error::WrongKind { .. } => ErrorKind::Invalid,
            _ => ErrorKind::Db,
        };
        groove_types::Error::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
