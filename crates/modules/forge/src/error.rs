use groove_types::ErrorKind;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Http(#[from] groove_http::Error),
    #[error("{host} refused the query: {message}")]
    Refused { host: String, message: String },
    #[error("{0}")]
    Invalid(String),
}

impl From<Error> for groove_types::Error {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error::Http(_) | Error::Refused { .. } => ErrorKind::Forge,
            Error::Invalid(_) => ErrorKind::Invalid,
        };
        groove_types::Error::new(kind, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
