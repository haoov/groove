//! The error every `#[tauri::command]` returns. Domain code keeps `anyhow`; this type is
//! the boundary, so the frontend gets a kind it can branch on instead of a bare string.

use crate::core::db::error::StoreError;

/// What went wrong, coarsely. The UI branches on this; the message is for the human.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// No row, file or ref by that name.
    NotFound,
    /// The request cannot apply to this state: a dirty worktree, an existing branch.
    Conflict,
    /// The caller asked for something malformed.
    Invalid,
    /// A git subprocess failed.
    Git,
    /// A forge call failed: GitLab, GitHub, or their CLI.
    Forge,
    /// A task source failed: Notion, GitHub Issues.
    Provider,
    /// Local disk.
    Io,
    /// The database.
    Db,
    /// The agent, its PTY, or the MCP server.
    Agent,
    /// Anything with no better kind.
    Internal,
}

#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Invalid, message)
    }
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    /// Re-label an error whose kind the call site knows better than its source did.
    pub fn with_kind(mut self, kind: ErrorKind) -> Self {
        self.kind = kind;
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        let kind = match &e {
            StoreError::NotFound { .. } => ErrorKind::NotFound,
            StoreError::Sqlx(_) => ErrorKind::Db,
        };
        Self::new(kind, e.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::new(ErrorKind::Db, e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        let kind = match e.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            _ => ErrorKind::Io,
        };
        Self::new(kind, e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorKind::Invalid, e.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::new(ErrorKind::Internal, e.to_string())
    }
}

/// `anyhow` carries the whole context chain as its message; the kind comes from whatever
/// typed error is still in the chain.
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        let message = format!("{e:#}");
        if let Some(labelled) = e.downcast_ref::<AppError>() {
            return Self::new(labelled.kind, message);
        }
        if let Some(store) = e.downcast_ref::<StoreError>() {
            return Self::new(AppError::from_store_kind(store), message);
        }
        if e.downcast_ref::<crate::core::forge::auth::CliMissing>()
            .is_some()
        {
            return Self::new(ErrorKind::Forge, message);
        }
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            if io.kind() == std::io::ErrorKind::NotFound {
                return Self::new(ErrorKind::NotFound, message);
            }
            return Self::new(ErrorKind::Io, message);
        }
        if e.downcast_ref::<sqlx::Error>().is_some() {
            return Self::new(ErrorKind::Db, message);
        }
        Self::new(ErrorKind::Internal, message)
    }
}

impl AppError {
    fn from_store_kind(e: &StoreError) -> ErrorKind {
        match e {
            StoreError::NotFound { .. } => ErrorKind::NotFound,
            StoreError::Sqlx(_) => ErrorKind::Db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_miss_keeps_its_kind_through_anyhow() {
        let store = StoreError::not_found("worktree", "wt-1");
        let direct: AppError = store.into();
        assert_eq!(direct.kind, ErrorKind::NotFound);
        assert_eq!(direct.message, "no worktree wt-1");

        let wrapped: AppError = anyhow::Error::from(StoreError::not_found("session", "s-1"))
            .context("opening the task")
            .into();
        assert_eq!(wrapped.kind, ErrorKind::NotFound);
        assert!(wrapped.message.contains("opening the task"), "{wrapped}");
        assert!(wrapped.message.contains("no session s-1"), "{wrapped}");
    }

    /// Domain code stays on `anyhow` and still names the kind.
    #[test]
    fn a_labelled_refusal_keeps_its_kind_through_anyhow() {
        let e: AppError = anyhow::Error::from(AppError::conflict("worktree has changes"))
            .context("closing the worktree")
            .into();
        assert_eq!(e.kind, ErrorKind::Conflict);
        assert!(e.message.contains("worktree has changes"), "{e}");
    }

    #[test]
    fn an_unlabelled_failure_is_internal() {
        let e: AppError = anyhow::anyhow!("something broke").into();
        assert_eq!(e.kind, ErrorKind::Internal);
        assert_eq!(e.message, "something broke");
    }

    #[test]
    fn a_missing_file_reports_not_found() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: AppError = io.into();
        assert_eq!(e.kind, ErrorKind::NotFound);
    }

    /// The wire shape the frontend reads.
    #[test]
    fn it_serializes_as_kind_and_message() {
        let json = serde_json::to_value(AppError::conflict("worktree has changes")).unwrap();
        assert_eq!(json["kind"], "conflict");
        assert_eq!(json["message"], "worktree has changes");
    }
}
