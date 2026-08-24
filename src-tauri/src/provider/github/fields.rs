//! Writing board fields and the issue itself.

use crate::core::config::GithubConfig;
use crate::core::forge::api;

const FIELD_IDS: &str = r#"
query($project: ID!) {
  node(id: $project) {
    ... on ProjectV2 {
      fields(first: 50) {
        nodes {
          ... on ProjectV2FieldCommon { id name dataType }
          ... on ProjectV2SingleSelectField { options { id name } }
          ... on ProjectV2IterationField {
            configuration { iterations { id title } completedIterations { id title } }
          }
        }
      }
    }
  }
}
"#;

const SET_FIELD: &str = r#"
mutation($project: ID!, $item: ID!, $field: ID!, $value: ProjectV2FieldValue!) {
  updateProjectV2ItemFieldValue(
    input: { projectId: $project, itemId: $item, fieldId: $field, value: $value }
  ) { projectV2Item { id } }
}
"#;

const CLEAR_FIELD: &str = r#"
mutation($project: ID!, $item: ID!, $field: ID!) {
  clearProjectV2ItemFieldValue(
    input: { projectId: $project, itemId: $item, fieldId: $field }
  ) { projectV2Item { id } }
}
"#;

pub(super) struct FieldDef {
    pub id: String,
    pub data_type: String,
    /// Option name -> option id, for the types whose writes take an id.
    pub options: Vec<(String, String)>,
}

pub(super) async fn field_def(
    cfg: &GithubConfig,
    project_id: &str,
    name: &str,
) -> anyhow::Result<FieldDef> {
    let res = api::github_graphql(
        &cfg.host,
        FIELD_IDS,
        serde_json::json!({ "project": project_id }),
    )
    .await?;

    for f in res["data"]["node"]["fields"]["nodes"].as_array().unwrap_or(&vec![]) {
        if f["name"].as_str() != Some(name) {
            continue;
        }
        let mut options: Vec<(String, String)> = f["options"]
            .as_array()
            .map(|o| {
                o.iter()
                    .filter_map(|x| Some((x["name"].as_str()?.to_string(), x["id"].as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        for key in ["iterations", "completedIterations"] {
            if let Some(iters) = f["configuration"][key].as_array() {
                options.extend(iters.iter().filter_map(|i| {
                    Some((i["title"].as_str()?.to_string(), i["id"].as_str()?.to_string()))
                }));
            }
        }
        return Ok(FieldDef {
            id: f["id"].as_str().unwrap_or_default().to_string(),
            data_type: f["dataType"].as_str().unwrap_or("TEXT").to_string(),
            options,
        });
    }
    anyhow::bail!("{name} is not a field on this board")
}

/// The mutation payload for a value, in the shape its field type takes.
///
/// Single-select and iteration are addressed by option id, not by name — which is
/// why the schema carries ids at all.
fn field_value(def: &FieldDef, value: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let by_name = |name: &str| -> anyhow::Result<String> {
        def.options
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, id)| id.clone())
            .ok_or_else(|| {
                let known: Vec<&str> = def.options.iter().map(|(n, _)| n.as_str()).collect();
                anyhow::anyhow!("{name} is not one of {known:?}")
            })
    };

    Ok(match def.data_type.as_str() {
        "SINGLE_SELECT" => {
            let name = value.as_str().ok_or_else(|| anyhow::anyhow!("expected an option name"))?;
            serde_json::json!({ "singleSelectOptionId": by_name(name)? })
        }
        "ITERATION" => {
            let name = value.as_str().ok_or_else(|| anyhow::anyhow!("expected an iteration"))?;
            serde_json::json!({ "iterationId": by_name(name)? })
        }
        "NUMBER" => {
            let n = value.as_f64().ok_or_else(|| anyhow::anyhow!("expected a number"))?;
            serde_json::json!({ "number": n })
        }
        "DATE" => {
            let d = value.as_str().ok_or_else(|| anyhow::anyhow!("expected a date"))?;
            serde_json::json!({ "date": d })
        }
        _ => {
            let t = value.as_str().unwrap_or_default();
            serde_json::json!({ "text": t })
        }
    })
}

/// Set one board field. A null value clears it.
pub(super) async fn set_field(
    cfg: &GithubConfig,
    project_id: &str,
    item_id: &str,
    name: &str,
    value: &serde_json::Value,
) -> anyhow::Result<String> {
    let def = field_def(cfg, project_id, name).await?;

    if value.is_null() {
        let vars = serde_json::json!({ "project": project_id, "item": item_id, "field": def.id });
        api::github_graphql(&cfg.host, CLEAR_FIELD, vars).await?;
        return Ok(String::new());
    }

    let payload = field_value(&def, value)?;
    let vars = serde_json::json!({
        "project": project_id, "item": item_id, "field": def.id, "value": payload,
    });
    api::github_graphql(&cfg.host, SET_FIELD, vars).await?;

    Ok(value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn select() -> FieldDef {
        FieldDef {
            id: "PVTSSF_x".into(),
            data_type: "SINGLE_SELECT".into(),
            options: vec![
                ("Ready".into(), "61e4505c".into()),
                ("In progress".into(), "47fc9ee4".into()),
            ],
        }
    }

    /// The schema hands the frontend the option NAME as the value to send, and the
    /// mutation takes the option id — so the resolution happens here. Sending what
    /// the frontend sends must work; sending a node id must not silently pass.
    #[test]
    fn a_select_is_addressed_by_option_id() {
        let out = field_value(&select(), &serde_json::json!("In progress")).unwrap();
        assert_eq!(out, serde_json::json!({ "singleSelectOptionId": "47fc9ee4" }));
    }

    #[test]
    fn an_unknown_option_names_the_ones_that_exist() {
        let err = field_value(&select(), &serde_json::json!("Shipped")).unwrap_err().to_string();
        assert!(err.contains("Ready") && err.contains("In progress"), "{err}");
    }

    /// What the frontend actually sends is PropertyOption.id, which the schema
    /// sets to the option name. This is the round trip that was broken.
    #[test]
    fn the_value_the_schema_offers_is_the_value_a_write_accepts() {
        let def = select();
        for (id_the_ui_sends, expected) in
            [("Ready", "61e4505c"), ("In progress", "47fc9ee4")]
        {
            let out = field_value(&def, &serde_json::json!(id_the_ui_sends)).unwrap();
            assert_eq!(out, serde_json::json!({ "singleSelectOptionId": expected }));
        }
    }

    #[test]
    fn a_number_field_takes_a_number() {
        let def = FieldDef { id: "f".into(), data_type: "NUMBER".into(), options: vec![] };
        assert_eq!(
            field_value(&def, &serde_json::json!(2.5)).unwrap(),
            serde_json::json!({ "number": 2.5 })
        );
        assert!(field_value(&def, &serde_json::json!("2.5")).is_err());
    }

    #[test]
    fn anything_else_is_text() {
        let def = FieldDef { id: "f".into(), data_type: "TEXT".into(), options: vec![] };
        assert_eq!(
            field_value(&def, &serde_json::json!("hello")).unwrap(),
            serde_json::json!({ "text": "hello" })
        );
    }
}

/// The label this board uses for an intent.
///
/// Read off the board rather than the config: each board names its own columns,
/// and with no board nominated there is no single vocabulary to have detected at
/// setup. The config value is the fallback for a board whose names match nothing.
pub(super) async fn status_for(
    cfg: &GithubConfig,
    project_id: &str,
    intent: crate::provider::types::StatusIntent,
) -> String {
    use crate::provider::types::StatusIntent;

    let configured = match intent {
        StatusIntent::Ready => &cfg.status_map.ready,
        StatusIntent::InProgress => &cfg.status_map.in_progress,
        StatusIntent::Done => &cfg.status_map.done,
    };

    let Ok(def) = field_def(cfg, project_id, &cfg.properties.status).await else {
        return configured.clone();
    };
    let options: Vec<String> = def.options.iter().map(|(n, _)| n.clone()).collect();
    let detected = crate::provider::detect::detect_status_map(&crate::provider::types::TaskSchema {
        database_id: project_id.to_string(),
        title_property: "Title".into(),
        properties: vec![crate::provider::types::PropertySchema {
            name: cfg.properties.status.clone(),
            kind: "status".into(),
            options: options
                .iter()
                .map(crate::provider::types::PropertyOption::named)
                .collect(),
            relation_db: None,
            editable: true,
            meta: false,
        }],
        status_groups: vec![],
        hours_property: None,
    });

    let picked = match intent {
        StatusIntent::Ready => detected.ready,
        StatusIntent::InProgress => detected.in_progress,
        StatusIntent::Done => detected.done,
    };
    if picked.is_empty() { configured.clone() } else { picked }
}
