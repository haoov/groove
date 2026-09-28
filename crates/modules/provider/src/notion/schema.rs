//! The database's properties, each with its type and, for a choice, its values.

use groove_types::{Kind, Property};
use serde_json::Value;

use super::Notion;
use crate::Result;

impl Notion {
    /// Every property of the task database, by name.
    pub async fn schema(&self) -> Result<Vec<Property>> {
        let database = self.database(&self.config.database_id).await?;
        Ok(properties(&database))
    }
}

pub(crate) fn properties(database: &Value) -> Vec<Property> {
    let each = database["properties"].as_object().into_iter().flatten();
    let mut out: Vec<Property> = each
        .map(|(name, def)| {
            let kind = def["type"].as_str().unwrap_or("");
            let options = def[kind]["options"].as_array().into_iter().flatten();
            Property {
                name: name.clone(),
                kind: kind_of(kind),
                options: options
                    .filter_map(|one| one["name"].as_str().map(str::to_string))
                    .collect(),
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn kind_of(kind: &str) -> Kind {
    match kind {
        "status" => Kind::Status,
        "select" => Kind::Select,
        "date" => Kind::Date,
        "number" => Kind::Number,
        "people" => Kind::People,
        "relation" => Kind::Relation,
        _ => Kind::Other,
    }
}
