#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("{method} {url} failed {status}: {detail}")]
    Status {
        method: String,
        url: String,
        status: u16,
        detail: String,
    },
    #[error("{method} {url} was refused: the token is not accepted")]
    Unauthorized { method: String, url: String },
    #[error("{url} returned a body that is not the expected JSON: {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("no token: {0}")]
    Auth(String),
    #[error("{url} refused the query: {message}")]
    Refused { url: String, message: String },
}

pub type Result<T> = std::result::Result<T, Error>;
