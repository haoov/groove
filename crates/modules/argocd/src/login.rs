//! The token the argocd CLI keeps for its current context after `argocd login --sso`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A token Argo CD signs a session with; it never prints.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// The cookie Argo CD's web UI sends it in; the API server keeps `Authorization` for itself.
    pub fn cookie(&self) -> String {
        format!("argocd.token={}", self.0)
    }

    /// The token itself, for a header or a cookie to carry.
    pub fn secret(&self) -> &str {
        &self.0
    }

    /// When it expires, in seconds since the epoch; none when it reads as no JWT.
    pub fn expires(&self) -> Option<i64> {
        let payload = self.0.split('.').nth(1)?;
        let claims: serde_json::Value = serde_json::from_slice(&base64url(payload)?).ok()?;
        claims["exp"].as_i64()
    }
}

/// The CLI call that refreshes an expired SSO token before it asks anything: the lightest one.
pub const REFRESH: [&str; 2] = ["account", "get-user-info"];

/// Unpadded base64url, as a JWT's parts are written.
fn base64url(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut held, mut bits) = (0u32, 0);
    for c in text.trim_end_matches('=').bytes() {
        held = (held << 6) | u32::from(value(c)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((held >> bits) as u8);
            held &= (1 << bits) - 1;
        }
    }
    Some(out)
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(…)")
    }
}

/// The CLI's config file under `home`, as the CLI itself reads it.
pub fn config_path(home: &Path) -> PathBuf {
    home.join(".config").join("argocd").join("config")
}

type Item = BTreeMap<String, String>;

/// The current context's server and its user's token; none when either is missing.
pub fn token_of(config: &str) -> Option<(String, Token)> {
    let read = sections(config);
    let current = read.top.get("current-context")?;
    let context = read
        .items("contexts")
        .find(|one| one.get("name") == Some(current))?;
    let user = context.get("user")?;
    let entry = read
        .items("users")
        .find(|one| one.get("name") == Some(user))?;
    let token = entry.get("auth-token").filter(|one| !one.is_empty())?;
    let server = context.get("server").cloned().unwrap_or_default();
    Some((server, Token(token.clone())))
}

/// The file's top-level keys, and each list's items by their keys.
#[derive(Default)]
struct Read {
    top: Item,
    lists: BTreeMap<String, Vec<Item>>,
}

impl Read {
    fn items<'a>(&'a self, list: &str) -> impl Iterator<Item = &'a Item> {
        self.lists.get(list).into_iter().flatten()
    }
}

/// The CLI writes flat YAML: top-level keys, and lists of one-level maps.
fn sections(config: &str) -> Read {
    let mut read = Read::default();
    let mut list = String::new();
    for line in config.lines().filter(|one| !one.trim().is_empty()) {
        let indented = line.starts_with(' ') || line.starts_with('-');
        let Some((key, value)) = pair(line.trim_start_matches(['-', ' '])) else {
            continue;
        };
        if !indented {
            if value.is_empty() {
                list = key;
            } else {
                read.top.insert(key, value);
            }
            continue;
        }
        let items = read.lists.entry(list.clone()).or_default();
        if line.trim_start().starts_with('-') || items.is_empty() {
            items.push(Item::new());
        }
        if let Some(item) = items.last_mut() {
            item.insert(key, value);
        }
    }
    read
}

fn pair(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once(':')?;
    let value = value.trim().trim_matches('"');
    Some((key.trim().to_string(), value.to_string()))
}
