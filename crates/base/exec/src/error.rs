use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{program} timed out after {}s", after.as_secs())]
    TimedOut { program: String, after: Duration },
    #[error("{program} failed: {stderr}")]
    Failed { program: String, stderr: String },
    #[error("pty: {0}")]
    Pty(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
