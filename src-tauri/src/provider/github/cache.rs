//! Short-lived caches for the two GitHub reads everything else is built on.
//!
//! The assigned-issue search is one query for the whole queue, and a single-task
//! operation used to re-run it just to pick one item out — so editing one property
//! cost two full searches. The list response already carries everything a task
//! operation needs, so it is worth holding briefly.
//!
//! Deliberately short, and deliberately not used by `list_tasks`: an explicit
//! refresh must always hit GitHub, or the button would lie.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use super::projects::BoardItem;

/// Long enough to cover one interaction, short enough that a change made on
/// GitHub shows up without waiting.
const TTL_SECS: i64 = 20;

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

type Issues = RwLock<Option<(i64, Vec<BoardItem>)>>;
type Fields = RwLock<HashMap<String, (i64, serde_json::Value)>>;

fn issues_cell() -> &'static Issues {
    static C: OnceLock<Issues> = OnceLock::new();
    C.get_or_init(|| RwLock::new(None))
}

fn fields_cell() -> &'static Fields {
    static C: OnceLock<Fields> = OnceLock::new();
    C.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(super) fn issues() -> Option<Vec<BoardItem>> {
    let guard = issues_cell().read().ok()?;
    let (at, items) = guard.as_ref()?;
    (now() - at < TTL_SECS).then(|| items.clone())
}

pub(super) fn put_issues(items: &[BoardItem]) {
    if let Ok(mut guard) = issues_cell().write() {
        *guard = Some((now(), items.to_vec()));
    }
}

pub(super) fn fields(project_id: &str) -> Option<serde_json::Value> {
    let guard = fields_cell().read().ok()?;
    let (at, value) = guard.get(project_id)?;
    (now() - at < TTL_SECS).then(|| value.clone())
}

pub(super) fn put_fields(project_id: &str, value: serde_json::Value) {
    if let Ok(mut guard) = fields_cell().write() {
        guard.insert(project_id.to_string(), (now(), value));
    }
}

/// After a write: the next read must see what was just written, not the copy
/// taken before it.
pub(super) fn invalidate() {
    if let Ok(mut guard) = issues_cell().write() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(number: i64) -> BoardItem {
        BoardItem {
            item_id: "i".into(), project_id: "p".into(), board: "b".into(),
            owner: "haoov".into(), repo: "groove".into(), number,
            title: String::new(), url: String::new(), body: String::new(),
            labels: vec![], assignees: vec![], fields: vec![],
        }
    }

    /// One test, not three: the cache is process-global, so separate tests would
    /// race each other rather than the thing under test.
    #[test]
    fn the_list_is_reused_until_it_is_stale_or_a_write_drops_it() {
        put_issues(&[item(1)]);
        assert_eq!(issues().map(|i| i.len()), Some(1), "a fresh list is reused");

        invalidate();
        assert!(issues().is_none(), "a write must send the next read back to GitHub");

        if let Ok(mut g) = issues_cell().write() {
            *g = Some((now() - TTL_SECS - 1, vec![item(1)]));
        }
        assert!(issues().is_none(), "a stale list is not reused");
    }
}
