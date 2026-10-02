//! A new session of each kind, as it is first written down.

use groove_types::{Session, SessionId, SessionKind, Task, Timestamp};

/// The session that works a task: its short id is the session's own.
pub fn task_session(task: &Task, now: Timestamp) -> Session {
    Session {
        id: SessionId::new(task.short_id.clone()),
        title: task.title.clone(),
        kind: SessionKind::Task {
            external_id: task.external_id.clone(),
        },
        created_at: now,
    }
}

/// The session that reviews an MR: the same id every time, its own title.
pub fn review_session(mr: &groove_types::ReviewMr, now: Timestamp) -> Session {
    Session {
        id: SessionId::new(mr.session_id()),
        title: mr.title.clone(),
        kind: SessionKind::Review {
            project: mr.project.clone(),
            iid: mr.iid,
        },
        created_at: now,
    }
}

/// An explorer: no ticket yet, a title the user gave or the default.
pub fn explorer(title: Option<&str>, now: Timestamp) -> Session {
    let short = uuid::Uuid::new_v4().simple().to_string();
    let title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("Explorer")
        .to_string();
    Session {
        id: SessionId::new(format!("explorer-{}", &short[..8])),
        title,
        kind: SessionKind::Explorer,
        created_at: now,
    }
}

/// The session one standalone routine runs in: the same id every time, the routine's name.
pub fn routine_session(routine: &groove_types::Routine, now: Timestamp) -> Session {
    let slug: String = routine
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    Session {
        id: SessionId::new(format!("routine-{slug}")),
        title: routine.name.clone(),
        kind: SessionKind::Routine {
            routine: routine.id.clone(),
        },
        created_at: now,
    }
}
