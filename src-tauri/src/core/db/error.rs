#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("no {entity} {id}")]
    NotFound { entity: &'static str, id: String },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl StoreError {
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound { entity, id: id.into() }
    }
}

pub type StoreResult<T> = Result<T, StoreError>;
