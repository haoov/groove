//! The task shapes every provider speaks in.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertySchema {
    pub name: String,
    /// The shared type vocabulary — the frontend renders by this.
    pub kind: String,
    /// Allowed values for select / status / multi_select, in the provider's order.
    pub options: Vec<String>,
    /// Target database for a relation, so its rows can be offered as choices.
    pub relation_db: Option<String>,
    /// False for formulas, rollups and timestamps: displayable, not settable.
    pub editable: bool,
}

/// A status property's option groups, as the provider itself classifies them.
/// This is the ONLY non-guess signal for what an option means — "Fixed with
/// required action" is a completion state and no amount of name matching would
/// say so. Empty when the provider has no grouping.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct StatusGroup {
    pub name: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct TaskSchema {
    pub database_id: String,
    /// The title property's name — it differs per database ("Task name" here).
    pub title_property: String,
    pub properties: Vec<PropertySchema>,
    pub status_groups: Vec<StatusGroup>,
}

impl TaskSchema {
    pub fn property(&self, name: &str) -> Option<&PropertySchema> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// The database a relation property points at. `None` when the property is
    /// absent or isn't a relation.
    pub fn relation_target(&self, property: &str) -> Option<&str> {
        self.property(property)?.relation_db.as_deref()
    }
}

/// One choice for a relation property.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct RelationOption {
    pub id: String,
    pub title: String,
}

/// One property as the panel sees it: what it is, its current value, and a
/// human rendering for the types we can display but not edit.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertyValue {
    pub name: String,
    pub kind: String,
    /// Canonical value. `null` when unset.
    // `unknown` on the TS side: the default JsonValue binding imports from
    // outside the Vite root, and the frontend treats it as opaque anyway.
    #[ts(type = "unknown")]
    pub value: serde_json::Value,
    /// Read-only rendering, used for formulas, rollups, people, timestamps.
    pub display: String,
}
