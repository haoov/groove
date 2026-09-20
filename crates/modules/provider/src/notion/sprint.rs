//! Which sprint is running, and the pages that name it.

use groove_types::Timestamp;

/// How long the sprints the last read found still stand.
const FRESH: i64 = 300;

/// The sprint pages a read found, and when it found them.
#[derive(Debug, Default)]
pub(super) struct Sprints {
    held: std::sync::Mutex<Option<(Timestamp, Vec<String>)>>,
}

impl Sprints {
    pub(super) fn read(&self, now: Timestamp) -> Option<Vec<String>> {
        let held = self.held.lock().ok()?;
        let (at, ids) = held.as_ref()?;
        (now.seconds() - at.seconds() < FRESH).then(|| ids.clone())
    }

    pub(super) fn keep(&self, now: Timestamp, ids: Vec<String>) {
        if let Ok(mut held) = self.held.lock() {
            *held = Some((now, ids));
        }
    }
}

/// The database the relation of this name points at.
pub(super) fn target(database: &serde_json::Value, name: &str) -> Option<String> {
    let property = database["properties"].get(name)?;
    property["relation"]["database_id"]
        .as_str()
        .map(str::to_string)
}

/// The first property of a database that holds a status.
pub(super) fn status_property(database: &serde_json::Value) -> Option<String> {
    database["properties"]
        .as_object()?
        .iter()
        .find(|(_, value)| value["type"].as_str() == Some("status"))
        .map(|(name, _)| name.clone())
}

/// The pages of one query, by id.
pub(super) fn ids(reply: &serde_json::Value) -> Vec<String> {
    reply["results"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
