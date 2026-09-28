//! The fields of the boards your issues stand on, each once, their options merged.

use groove_types::{Kind, Property};
use serde_json::Value;

use super::{Github, query};
use crate::Result;

impl Github {
    /// Every field of every board an assigned issue is on, by name.
    pub async fn schema(&self) -> Result<Vec<Property>> {
        let reply = self
            .ask(&query::assigned(), serde_json::json!({ "after": null }))
            .await?;
        Ok(fields(&reply))
    }
}

pub(crate) fn fields(reply: &Value) -> Vec<Property> {
    let mut out: Vec<Property> = Vec::new();
    let issues = reply["data"]["search"]["nodes"]
        .as_array()
        .into_iter()
        .flatten();
    let items = issues.flat_map(|issue| array(&issue["projectItems"]["nodes"]));
    let fields = items.flat_map(|item| array(&item["project"]["fields"]["nodes"]));
    for field in fields {
        let Some(name) = field["name"].as_str() else {
            continue;
        };
        let options = array(&field["options"]).filter_map(|one| one["name"].as_str());
        let at = match out.iter().position(|one| one.name == name) {
            Some(at) => at,
            None => {
                out.push(Property {
                    name: name.to_string(),
                    kind: kind_of(field["dataType"].as_str().unwrap_or("")),
                    options: Vec::new(),
                });
                out.len() - 1
            }
        };
        for option in options {
            if !out[at].options.iter().any(|held| held == option) {
                out[at].options.push(option.to_string());
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn array(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

fn kind_of(kind: &str) -> Kind {
    match kind {
        "SINGLE_SELECT" => Kind::Select,
        "DATE" => Kind::Date,
        "NUMBER" => Kind::Number,
        _ => Kind::Other,
    }
}
