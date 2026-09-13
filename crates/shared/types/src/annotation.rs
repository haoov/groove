use crate::{AnnotationId, RepoId, SessionId, Timestamp};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    #[default]
    Open,
    Resolved,
}

/// A note on a range of the new side of a file; one line has `start == end`.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Annotation {
    pub id: AnnotationId,
    pub session: SessionId,
    pub repo: RepoId,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub author: String,
    pub status: AnnotationStatus,
    pub created_at: Timestamp,
}
