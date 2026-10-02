//! The `sessions` table as SQLite holds it, and the session it reads back as.

use groove_types::{ExternalId, Session, SessionId, SessionKind, Timestamp};

/// A `sessions` row as SQLite holds it: the kind and its nullable identity columns.
#[derive(sqlx::FromRow)]
pub(crate) struct SessionRow {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub external_id: Option<String>,
    pub review_project: Option<String>,
    pub review_iid: Option<i64>,
    pub created_at: i64,
    /// The routine it runs, from `routine_sessions`.
    pub routine: Option<String>,
}

impl From<SessionRow> for Session {
    fn from(row: SessionRow) -> Self {
        let kind = match (
            row.kind.as_str(),
            row.external_id,
            row.review_project,
            row.review_iid,
        ) {
            ("task", Some(external_id), _, _) => SessionKind::Task {
                external_id: ExternalId::new(external_id),
            },
            ("review", _, Some(project), Some(iid)) => SessionKind::Review {
                project,
                iid: iid.max(0) as u64,
            },
            _ => match row.routine {
                Some(routine) => SessionKind::Routine { routine },
                None => SessionKind::Explorer,
            },
        };
        Session {
            id: SessionId::new(row.id),
            title: row.title,
            kind,
            created_at: Timestamp::new(row.created_at),
        }
    }
}
