//! The Notion HTTP verbs. `pub(super)` on purpose: only this feature module can
//! reach the API, so every caller outside `notion/` goes through a named function.

use crate::core::timing::timed;

const NOTION_BASE: &str = "https://api.notion.com";
const NOTION_VERSION: &str = "2022-06-28";

/// One page of results per request; Notion's maximum.
const PAGE_SIZE: usize = 100;

async fn call(
    method: reqwest::Method,
    token: &str,
    path: &str,
    body: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let verb = method.as_str().to_string();
    timed("http", format!("notion {verb} {path}"), async {
        let mut req = crate::core::http::client()
            .request(method, format!("{NOTION_BASE}/{path}"))
            .bearer_auth(token)
            .header("Notion-Version", NOTION_VERSION);
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let out: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("Notion {verb} {path} failed {status}: {out}"));
        }
        Ok(out)
    })
    .await
}

pub(super) async fn get(token: &str, path: &str) -> anyhow::Result<serde_json::Value> {
    call(reqwest::Method::GET, token, path, None).await
}

pub(super) async fn post(
    token: &str,
    path: &str,
    body: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    call(reqwest::Method::POST, token, path, Some(body)).await
}

pub(super) async fn patch(
    token: &str,
    path: &str,
    body: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    call(reqwest::Method::PATCH, token, path, Some(body)).await
}

/// Walk a GET endpoint's cursor pagination and return every `results` entry.
/// `path` must carry no query string. `max_pages` is a runaway backstop, far
/// above any real dataset.
pub(super) async fn paginate_get(
    token: &str,
    path: &str,
    max_pages: usize,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut out: Vec<serde_json::Value> = vec![];
    let mut cursor: Option<String> = None;
    for _ in 0..max_pages {
        let page_path = match &cursor {
            Some(c) => format!("{path}?page_size={PAGE_SIZE}&start_cursor={c}"),
            None => format!("{path}?page_size={PAGE_SIZE}"),
        };
        let page = get(token, &page_path).await?;
        if !collect(&page, &mut out, &mut cursor) {
            break;
        }
    }
    Ok(out)
}

/// Same walk for a POST query endpoint; the cursor rides in the body.
pub(super) async fn paginate_post(
    token: &str,
    path: &str,
    base_body: &serde_json::Value,
    max_pages: usize,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut out: Vec<serde_json::Value> = vec![];
    let mut cursor: Option<String> = None;
    for _ in 0..max_pages {
        let mut body = base_body.clone();
        body["page_size"] = serde_json::json!(PAGE_SIZE);
        if let Some(c) = &cursor {
            body["start_cursor"] = serde_json::json!(c);
        }
        let page = post(token, path, &body).await?;
        if !collect(&page, &mut out, &mut cursor) {
            break;
        }
    }
    Ok(out)
}

/// Append one page's results; true when another page must be fetched.
/// `has_more` without a cursor would loop forever, so it counts as the end.
fn collect(
    page: &serde_json::Value,
    out: &mut Vec<serde_json::Value>,
    cursor: &mut Option<String>,
) -> bool {
    if let Some(results) = page["results"].as_array() {
        out.extend(results.iter().cloned());
    }
    match page["next_cursor"].as_str() {
        Some(next) if page["has_more"].as_bool() == Some(true) => {
            *cursor = Some(next.to_string());
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    /// The API host may be named nowhere else: outside this file, Notion is
    /// reached through named functions only. Same style as the other guards.
    #[test]
    fn no_notion_api_outside_notion_api() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(offenders.is_empty(), "api.notion.com outside provider/notion/api.rs: {offenders:?}");

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().ends_with("/provider/notion/api.rs")
                    && std::fs::read_to_string(&path).unwrap().contains("api.notion.com")
                {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
