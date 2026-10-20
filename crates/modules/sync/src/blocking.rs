//! Work that blocks, run on a thread of its own.

use groove_types::{Error, ErrorKind, Result};

pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    match tokio::task::spawn_blocking(work).await {
        Ok(done) => done,
        Err(e) => Err(Error::new(ErrorKind::Io, e.to_string())),
    }
}
