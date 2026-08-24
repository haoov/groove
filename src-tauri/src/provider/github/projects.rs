//! Which issues are tasks, and the board fields they carry.
//!
//! Issue-first, not board-first: a task is an open issue assigned to you that
//! somebody has put on a board. Asking that way needs no board configured, costs
//! one query rather than one per board, and a new board starts working on its own.

use crate::core::config::GithubConfig;
use crate::core::forge::api;

/// A board with more items than this is paginated; the cap stops one runaway
/// board from stalling the whole queue.
const MAX_PAGES: usize = 20;

const ASSIGNED: &str = r#"
query($after: String) {
  search(query: "assignee:@me is:issue is:open", type: ISSUE, first: 50, after: $after) {
    pageInfo { hasNextPage endCursor }
    nodes {
      ... on Issue {
        number title url body
        repository { name owner { login } }
        assignees(first: 10) { nodes { login } }
        labels(first: 20) { nodes { name } }
        projectItems(first: 5) {
          nodes {
            id
            project { id title }
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
}
"#;

/// One board item that is a task: an open issue assigned to the viewer.
pub(super) struct BoardItem {
    /// Addresses a field write, together with `project_id`.
    pub item_id: String,
    pub project_id: String,
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
    /// the name.
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

/// Every task: an open issue assigned to you that sits on some board.
///
/// An issue on no board is deliberately not a task — that is the whole filter, and
/// it is what "has a project" meant. When an issue is on several, the first one
/// GitHub returns supplies its fields; the board is recorded so the choice is
/// visible rather than silently flipping between syncs.
pub(super) async fn assigned_issues(cfg: &GithubConfig) -> anyhow::Result<Vec<BoardItem>> {
    let mut out = Vec::new();
    let mut after = serde_json::Value::Null;

    for _ in 0..MAX_PAGES {
        let res =
            api::github_graphql(&cfg.host, ASSIGNED, serde_json::json!({ "after": after })).await?;
        let search = &res["data"]["search"];

        for issue in search["nodes"].as_array().unwrap_or(&vec![]) {
            let Some(item) = issue["projectItems"]["nodes"].as_array().and_then(|n| n.first())
            else {
                continue;
            };

            out.push(BoardItem {
                item_id: item["id"].as_str().unwrap_or_default().to_string(),
                project_id: item["project"]["id"].as_str().unwrap_or_default().to_string(),
                board: item["project"]["title"].as_str().unwrap_or_default().to_string(),
                owner: issue["repository"]["owner"]["login"].as_str().unwrap_or_default().to_string(),
                repo: issue["repository"]["name"].as_str().unwrap_or_default().to_string(),
                number: issue["number"].as_i64().unwrap_or_default(),
                title: issue["title"].as_str().unwrap_or_default().to_string(),
                url: issue["url"].as_str().unwrap_or_default().to_string(),
                body: issue["body"].as_str().unwrap_or_default().to_string(),
                labels: issue["labels"]["nodes"]
                    .as_array()
                    .map(|l| l.iter().filter_map(|n| n["name"].as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                assignees: issue["assignees"]["nodes"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|n| n["login"].as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                fields: item["fieldValues"]["nodes"]
                    .as_array()
                    .map(|f| f.iter().filter_map(field_value).collect())
                    .unwrap_or_default(),
            });
        }

        if !search["pageInfo"]["hasNextPage"].as_bool().unwrap_or(false) {
            break;
        }
        after = search["pageInfo"]["endCursor"].clone();
    }

    Ok(out)
}

const ASSIGNED_COUNT: &str = r#"
query { search(query: "assignee:@me is:issue is:open", type: ISSUE, first: 1) { issueCount } }
"#;

/// Every open issue assigned to you, boarded or not.
pub(super) async fn assigned_count(cfg: &GithubConfig) -> anyhow::Result<i64> {
    let res = api::github_graphql(&cfg.host, ASSIGNED_COUNT, serde_json::json!({})).await?;
    Ok(res["data"]["search"]["issueCount"].as_i64().unwrap_or(0))
}

const VIEWER: &str = r#"query { viewer { login } }"#;

/// The signed-in login, which issue creation needs so the queue can find it again.
pub(super) async fn viewer_login(cfg: &GithubConfig) -> anyhow::Result<String> {
    let res = api::github_graphql(&cfg.host, VIEWER, serde_json::json!({})).await?;
    res["data"]["viewer"]["login"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("GitHub did not say who you are signed in as"))
}
