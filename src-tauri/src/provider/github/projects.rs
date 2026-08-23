//! The Projects v2 board query: which issues are tasks, and their field values.

use crate::core::config::GithubConfig;
use crate::core::forge::api;

/// A board with more items than this is paginated; the cap stops one runaway
/// board from stalling the whole queue.
const MAX_PAGES: usize = 20;

const ITEMS: &str = r#"
query($project: ID!, $after: String) {
  viewer { login }
  node(id: $project) {
    ... on ProjectV2 {
      title
      items(first: 50, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          content {
            __typename
            ... on Issue {
              number title url state body
              repository { name owner { login } }
              assignees(first: 10) { nodes { login } }
              labels(first: 20) { nodes { name } }
            }
          }
          fieldValues(first: 25) {
            nodes {
              __typename
              ... on ProjectV2ItemFieldSingleSelectValue {
                optionId name field { ... on ProjectV2SingleSelectField { name } }
              }
              ... on ProjectV2ItemFieldTextValue {
                text field { ... on ProjectV2Field { name } }
              }
              ... on ProjectV2ItemFieldNumberValue {
                number field { ... on ProjectV2Field { name } }
              }
              ... on ProjectV2ItemFieldDateValue {
                date field { ... on ProjectV2Field { name } }
              }
              ... on ProjectV2ItemFieldIterationValue {
                title iterationId field { ... on ProjectV2IterationField { name } }
              }
            }
          }
        }
      }
    }
  }
}
"#;

/// One board item that is a task: an open issue assigned to the viewer.
pub(super) struct BoardItem {
    /// Needed to address a field write; unused until writes land.
    #[allow(dead_code)]
    pub item_id: String,
    #[allow(dead_code)]
    pub board: String,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    pub body: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    /// Field name -> value, as the board reports it.
    pub fields: Vec<FieldValue>,
}

pub(super) struct FieldValue {
    pub name: String,
    pub value: serde_json::Value,
    pub display: String,
    /// The option id, which a single-select or iteration write takes instead of
    /// the name. Unused until writes land.
    #[allow(dead_code)]
    pub option_id: Option<String>,
}

fn field_value(node: &serde_json::Value) -> Option<FieldValue> {
    let name = node["field"]["name"].as_str()?.to_string();
    let (value, display, option_id) = match node["__typename"].as_str()? {
        "ProjectV2ItemFieldSingleSelectValue" => {
            let n = node["name"].as_str().unwrap_or_default().to_string();
            (
                serde_json::json!(n),
                n,
                node["optionId"].as_str().map(str::to_string),
            )
        }
        "ProjectV2ItemFieldIterationValue" => {
            let t = node["title"].as_str().unwrap_or_default().to_string();
            (
                serde_json::json!(t),
                t,
                node["iterationId"].as_str().map(str::to_string),
            )
        }
        "ProjectV2ItemFieldTextValue" => {
            let t = node["text"].as_str().unwrap_or_default().to_string();
            (serde_json::json!(t), t, None)
        }
        "ProjectV2ItemFieldNumberValue" => {
            let n = node["number"].as_f64();
            (
                serde_json::json!(n),
                n.map(|v| v.to_string()).unwrap_or_default(),
                None,
            )
        }
        "ProjectV2ItemFieldDateValue" => {
            let d = node["date"].as_str().unwrap_or_default().to_string();
            (serde_json::json!(d), d, None)
        }
        _ => return None,
    };
    Some(FieldValue { name, value, display, option_id })
}

/// Every task on one board: open issues assigned to the viewer.
///
/// Pull requests and draft issues sit on the same board and are skipped, as are
/// issues assigned to somebody else — a board is a team's, the queue is yours.
pub(super) async fn board_items(
    cfg: &GithubConfig,
    project_id: &str,
) -> anyhow::Result<Vec<BoardItem>> {
    let mut out = Vec::new();
    let mut after = serde_json::Value::Null;

    for _ in 0..MAX_PAGES {
        let vars = serde_json::json!({ "project": project_id, "after": after });
        let res = api::github_graphql(&cfg.host, ITEMS, vars).await?;
        let viewer = res["data"]["viewer"]["login"].as_str().unwrap_or_default().to_string();
        let node = &res["data"]["node"];
        let board = node["title"].as_str().unwrap_or_default().to_string();
        let items = &node["items"];

        for item in items["nodes"].as_array().unwrap_or(&vec![]) {
            let content = &item["content"];
            if content["__typename"].as_str() != Some("Issue")
                || content["state"].as_str() != Some("OPEN")
            {
                continue;
            }
            let assignees: Vec<String> = content["assignees"]["nodes"]
                .as_array()
                .map(|a| a.iter().filter_map(|n| n["login"].as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            if cfg.filters.filter_by_assignee && !assignees.iter().any(|a| *a == viewer) {
                continue;
            }

            out.push(BoardItem {
                item_id: item["id"].as_str().unwrap_or_default().to_string(),
                board: board.clone(),
                owner: content["repository"]["owner"]["login"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                repo: content["repository"]["name"].as_str().unwrap_or_default().to_string(),
                number: content["number"].as_i64().unwrap_or_default(),
                title: content["title"].as_str().unwrap_or_default().to_string(),
                url: content["url"].as_str().unwrap_or_default().to_string(),
                body: content["body"].as_str().unwrap_or_default().to_string(),
                labels: content["labels"]["nodes"]
                    .as_array()
                    .map(|l| l.iter().filter_map(|n| n["name"].as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                assignees,
                fields: item["fieldValues"]["nodes"]
                    .as_array()
                    .map(|f| f.iter().filter_map(field_value).collect())
                    .unwrap_or_default(),
            });
        }

        if !items["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) {
            break;
        }
        after = items["pageInfo"]["endCursor"].clone();
    }

    Ok(out)
}
