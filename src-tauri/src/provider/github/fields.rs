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

/// The board's field definitions, cached: `board_schema` and every write want the
/// same payload, and it changes about as often as the board is redesigned.
pub(super) async fn board_fields(
    host: &str,
    project_id: &str,
) -> anyhow::Result<serde_json::Value> {
    if let Some(hit) = super::cache::fields(project_id) {
        return Ok(hit);
    }
    let res = api::github_graphql(
        host,
        FIELD_IDS,
        serde_json::json!({ "project": project_id }),
    )
    .await?;
    let nodes = res["data"]["node"]["fields"]["nodes"].clone();
    super::cache::put_fields(project_id, nodes.clone());
    Ok(nodes)
}

// `host` rather than the whole config: the setup preview runs before one exists.
pub(super) async fn field_def(
    host: &str,
    project_id: &str,
    name: &str,
) -> anyhow::Result<FieldDef> {
    let nodes = board_fields(host, project_id).await?;

    for f in nodes.as_array().unwrap_or(&vec![]) {
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
    let def = field_def(&cfg.host, project_id, name).await?;

    if value.is_null() {
        let vars = serde_json::json!({ "project": project_id, "item": item_id, "field": def.id });
        api::github_graphql(&cfg.host, CLEAR_FIELD, vars).await?;
        super::cache::invalidate();
        return Ok(String::new());
    }

    let payload = field_value(&def, value)?;
    let vars = serde_json::json!({
        "project": project_id, "item": item_id, "field": def.id, "value": payload,
    });
    api::github_graphql(&cfg.host, SET_FIELD, vars).await?;
    super::cache::invalidate();

    Ok(value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string()))
}

/// The label this board uses for an intent.
///
/// Read off the board rather than the config: each board names its own columns,
/// and with none nominated there is no single vocabulary to have detected at setup.
pub(super) async fn status_for(
    cfg: &GithubConfig,
    project_id: &str,
    intent: crate::provider::types::StatusIntent,
) -> String {
    use crate::provider::types::StatusIntent;

    let (hint, configured) = match intent {
        StatusIntent::Ready => ("ready", &cfg.status_map.ready),
        StatusIntent::InProgress => ("progress", &cfg.status_map.in_progress),
        StatusIntent::Done => ("done", &cfg.status_map.done),
    };

    let Ok(def) = field_def(&cfg.host, project_id, &cfg.properties.status).await else {
        return configured.clone();
    };
    let options: Vec<String> = def.options.iter().map(|(n, _)| n.clone()).collect();
    pick_status(&options, hint, configured)
}

/// The names of a board's Status columns, for the setup preview: the user must
/// see what the board calls its states before status_map can name one.
pub(super) async fn status_columns(host: &str, project_id: &str) -> Vec<String> {
    let Ok(nodes) = board_fields(host, project_id).await else { return vec![] };
    nodes
        .as_array()
        .and_then(|fields| {
            fields.iter().find(|f| {
                f["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("Status"))
            })
        })
        .and_then(|f| f["options"].as_array())
        .map(|opts| opts.iter().filter_map(|o| o["name"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// The error for a board whose status columns match neither the intent nor the
/// config. Lists the real columns: the fix is one line in the config file, and
/// the list is what tells the user what to write there.
pub(super) fn no_status_error(
    intent: crate::provider::types::StatusIntent,
    columns: &[String],
) -> anyhow::Error {
    let listed = match columns.is_empty() {
        true => "the board's columns could not be read".to_string(),
        false => format!("the board's columns are: {}", columns.join(", ")),
    };
    anyhow::anyhow!(
        "no status column on this board matches {intent:?} — {listed}.          Set github.status_map in the config file to one of them."
    )
}

/// The board column for an intent: one whose name looks like it, else the
/// configured label when the board really has a column by that name, else nothing.
///
/// Never positional. detect_status_map ends in `options.first()`, which would have
/// filed a new issue into whatever column happens to be leftmost.
fn pick_status(options: &[String], hint: &str, configured: &str) -> String {
    if let Some(m) = options.iter().find(|o| o.to_lowercase().contains(hint)) {
        return m.clone();
    }
    if options.iter().any(|o| o == configured) {
        return configured.to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user's fix is a config line; the error must name the real columns.
    #[test]
    fn the_no_status_error_names_every_column() {
        let cols = vec!["Todo".to_string(), "Doing".to_string(), "Shipped".to_string()];
        let msg = no_status_error(crate::provider::types::StatusIntent::Done, &cols).to_string();
        for c in &cols {
            assert!(msg.contains(c.as_str()), "{msg}");
        }
        assert!(msg.contains("status_map"), "{msg}");
        let empty = no_status_error(crate::provider::types::StatusIntent::Done, &[]).to_string();
        assert!(empty.contains("could not be read"), "{empty}");
    }

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

    fn board(options: &[&str]) -> Vec<String> {
        options.iter().map(|s| s.to_string()).collect()
    }

    /// A board whose Done column is called something else must still resolve by
    /// name — and a board with no matching column must resolve to nothing rather
    /// than to whichever option is first.
    #[test]
    fn a_status_is_matched_by_name_never_by_position() {
        assert!(pick_status(&board(&["Backlog", "Todo", "In Progress", "Done"]), "done", "Done")
            .eq("Done"));
        // No "ready"-ish column: "Backlog" is first, and must NOT be chosen.
        assert!(pick_status(&board(&["Backlog", "Todo", "Done"]), "ready", "Ready").is_empty());
        // Unless the configured label really is one of the columns.
        assert!(pick_status(&board(&["Backlog", "Ready", "Done"]), "ready", "Ready").eq("Ready"));
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
