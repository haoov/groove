//! The Platform team's firing alerts, asked of incident.io with the key the `inc` CLI holds.

mod parse;

#[cfg(test)]
mod tests;

use groove_http::{Client, Method};
use groove_token::{Cli, CliToken, Token};
use groove_types::{Alert, AlertsConfig, Error, ErrorKind, Result};
use serde_json::Value;

const API: &str = "https://api.incident.io";
/// The most alerts one page holds.
const PAGE: &str = "50";
/// The most pages one read follows.
const PAGES: usize = 40;
/// The `Team` alert attribute, and the value it holds for Platform.
pub const TEAM: &str = "01KBCK0ZK7PVJY42HRMRY1PWF7";
pub const PLATFORM: &str = "platform";

/// incident.io's API, its key asked of `inc` and held until it is refused.
pub struct Incident {
    base: String,
    client: Client,
    token: Token,
}

impl Incident {
    /// The API the user logged `inc` in to.
    pub fn logged_in() -> Result<Self> {
        Self::at(API, Token::Cli(CliToken::new(Cli::Inc, "api.incident.io")))
    }

    pub fn at(base: impl Into<String>, token: Token) -> Result<Self> {
        Ok(Self {
            base: base.into().trim_end_matches('/').to_string(),
            client: Client::new().map_err(failed)?,
            token,
        })
    }

    /// Every firing alert of the Platform team, the most urgent first, the newest first within a rank.
    pub async fn firing(&self, config: &AlertsConfig) -> Result<Vec<Alert>> {
        let query = vec![
            ("status[one_of]".to_string(), "firing".to_string()),
            ("page_size".to_string(), PAGE.to_string()),
            (format!("attributes[{TEAM}][one_of]"), PLATFORM.to_string()),
        ];
        let mut alerts = Vec::new();
        for page in self.pages("/v2/alerts", query).await? {
            let listed = page["alerts"].as_array().cloned().unwrap_or_default();
            alerts.extend(listed.iter().filter_map(|one| parse::alert(one, config)));
        }
        alerts.sort_by_key(|one| (one.rank(), std::cmp::Reverse(one.created_at)));
        Ok(alerts)
    }

    /// Every page of a list, following its `after` cursor.
    async fn pages(&self, path: &str, query: Vec<(String, String)>) -> Result<Vec<Value>> {
        let mut out = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..PAGES {
            let mut asked = query.clone();
            asked.extend(after.take().map(|cursor| ("after".to_string(), cursor)));
            let page = self.get(path, &asked).await?;
            after = page["pagination_meta"]["after"]
                .as_str()
                .filter(|cursor| !cursor.is_empty())
                .map(str::to_string);
            out.push(page);
            if after.is_none() {
                break;
            }
        }
        Ok(out)
    }

    async fn get(&self, path: &str, query: &[(String, String)]) -> Result<Value> {
        let url = format!("{}{path}{}", self.base, encoded(query));
        let request = self.client.request(Method::GET, url);
        request.send_authed_json(&self.token).await.map_err(failed)
    }
}

/// `?a=b&c=d`, each name and value percent-encoded; nothing when there is no pair.
fn encoded(query: &[(String, String)]) -> String {
    let pairs: Vec<String> = query
        .iter()
        .map(|(name, value)| format!("{}={}", escaped(name), escaped(value)))
        .collect();
    match pairs.is_empty() {
        true => String::new(),
        false => format!("?{}", pairs.join("&")),
    }
}

fn escaped(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn failed(e: groove_http::Error) -> Error {
    Error::new(ErrorKind::Provider, format!("incident.io: {e}"))
}
