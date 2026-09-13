/// The shared property vocabulary every provider maps into.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyKind {
    Title,
    Text,
    Number,
    Select,
    MultiSelect,
    Status,
    Date,
    People,
    Relation,
    Checkbox,
    Url,
    Formula,
    Timestamp,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PropertySchema {
    pub name: String,
    pub kind: PropertyKind,
    pub options: Vec<PropertyOption>,
    pub relation_db: Option<String>,
    pub editable: bool,
    /// An id, a timestamp or a computed value; hidden from the property strip.
    pub meta: bool,
}

/// One choice for a property: an option on a select, or a row of a relation.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PropertyOption {
    pub id: String,
    pub title: String,
}

impl PropertyOption {
    /// For a provider whose options are identified by their name.
    pub fn named(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            id: title.clone(),
            title,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct StatusGroup {
    pub name: String,
    pub options: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskSchema {
    pub database_id: String,
    pub title_property: String,
    pub properties: Vec<PropertySchema>,
    pub status_groups: Vec<StatusGroup>,
    pub hours_property: Option<String>,
}

impl TaskSchema {
    pub fn property(&self, name: &str) -> Option<&PropertySchema> {
        self.properties.iter().find(|p| p.name == name)
    }

    pub fn relation_target(&self, property: &str) -> Option<&str> {
        self.property(property)?.relation_db.as_deref()
    }
}

/// One property as the overview shows it: kind, canonical value, display text.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PropertyValue {
    pub name: String,
    pub kind: PropertyKind,
    pub value: serde_json::Value,
    pub display: String,
}
