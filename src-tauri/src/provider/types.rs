//! The task shapes every provider speaks in.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertySchema {
    pub name: String,
    /// The shared type vocabulary — the frontend renders by this.
    pub kind: String,
    /// Allowed values for select / status / multi_select, in the provider's order.
    ///
    /// Carries ids as well as titles because GitHub's Projects v2 mutations take
    /// the option id, not its name. Notion has no separate id and sets both.
    pub options: Vec<PropertyOption>,
    /// Target database for a relation, so its rows can be offered as choices.
    pub relation_db: Option<String>,
    /// False for formulas, rollups and timestamps: displayable, not settable.
    pub editable: bool,
    /// Not a field the user sets — an id, a timestamp, a computed value. Kept out
    /// of the property strip.
    pub meta: bool,
}

/// A status property's option groups, as the provider itself classifies them.
/// This is the ONLY non-guess signal for what an option means — "Fixed with
/// required action" is a completion state and no amount of name matching would
/// say so. Empty when the provider has no grouping.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct StatusGroup {
    pub name: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct TaskSchema {
    pub database_id: String,
    /// The title property's name — it differs per database ("Task name" here).
    pub title_property: String,
    pub properties: Vec<PropertySchema>,
    pub status_groups: Vec<StatusGroup>,
    /// The number field hours are logged to, when this source has one. None means
    /// time is tracked locally and written nowhere else.
    pub hours_property: Option<String>,
}

impl TaskSchema {
    pub fn property(&self, name: &str) -> Option<&PropertySchema> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// The database a relation property points at. `None` when the property is
    /// absent or isn't a relation.
    pub fn relation_target(&self, property: &str) -> Option<&str> {
        self.property(property)?.relation_db.as_deref()
    }
}

/// One choice for a property: an option on a select, or a row of a relation.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertyOption {
    pub id: String,
    pub title: String,
}

impl PropertyOption {
    /// For a provider whose options are identified by their name.
    pub fn named(title: impl Into<String>) -> Self {
        let title = title.into();
        Self { id: title.clone(), title }
    }
}

/// One property as the panel sees it: what it is, its current value, and a
/// human rendering for the types we can display but not edit.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertyValue {
    pub name: String,
    pub kind: String,
    /// Canonical value. `null` when unset.
    // `unknown` on the TS side: the default JsonValue binding imports from
    // outside the Vite root, and the frontend treats it as opaque anyway.
    #[ts(type = "unknown")]
    pub value: serde_json::Value,
    /// Read-only rendering, used for formulas, rollups, people, timestamps.
    pub display: String,
}

// ─── Provider identity ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
#[serde(rename_all = "lowercase")]
#[ts(rename_all = "lowercase")]
pub enum ProviderId {
    Notion,
    Github,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Notion => "notion",
            ProviderId::Github => "github",
        }
    }
}

/// A task's identity at its source. `external_id` is the stored string form;
/// the shapes are distinct enough that the provider is recoverable from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKey {
    Notion { page_id: String },
    Github { host: String, owner: String, repo: String, number: i64 },
}

impl TaskKey {
    pub fn provider(&self) -> ProviderId {
        match self {
            TaskKey::Notion { .. } => ProviderId::Notion,
            TaskKey::Github { .. } => ProviderId::Github,
        }
    }

    /// `<page-uuid>` or `<host>/<owner>/<repo>#<number>`.
    pub fn external_id(&self) -> String {
        match self {
            TaskKey::Notion { page_id } => page_id.clone(),
            TaskKey::Github { host, owner, repo, number } => {
                format!("{host}/{owner}/{repo}#{number}")
            }
        }
    }

    pub fn parse(external_id: &str) -> anyhow::Result<Self> {
        let Some((path, number)) = external_id.rsplit_once('#') else {
            // A Notion page id is a uuid. Accepting anything else here turns a
            // malformed id into a confusing 404 from Notion.
            if !is_uuid(external_id) {
                anyhow::bail!("not a task id: {external_id}");
            }
            return Ok(TaskKey::Notion { page_id: external_id.to_string() });
        };
        let parts: Vec<&str> = path.split('/').collect();
        let [host, owner, repo] = parts[..] else {
            anyhow::bail!("not a task id: {external_id}");
        };
        let number = number
            .parse()
            .map_err(|_| anyhow::anyhow!("not a task id: {external_id}"))?;
        Ok(TaskKey::Github {
            host: host.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            number,
        })
    }
}

/// Dashed or bare, which are both spellings Notion uses.
fn is_uuid(text: &str) -> bool {
    let bare: String = text.chars().filter(|c| *c != '-').collect();
    bare.len() == 32 && bare.chars().all(|c| c.is_ascii_hexdigit())
}

/// What the app asks for; the provider owns the label it writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum StatusIntent {
    Ready,
    InProgress,
    Done,
}

/// One task as its provider reports it, before the store mints a short_id.
#[derive(Debug, Clone)]
pub struct FetchedTask {
    pub key: TaskKey,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub url: String,
    /// The provider's own identifier, when it has one worth using ("PLAT-42").
    pub natural_short_id: Option<String>,
    /// Appended to branch names. Falls back to the short_id when absent.
    pub branch_tag: Option<String>,
    /// Which board supplied the fields, when the source has more than one place
    /// they could have come from.
    pub board: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskDraft<'a> {
    pub title: &'a str,
    pub body_markdown: &'a str,
    /// Where to file it, for a provider that needs telling. Notion has one
    /// database; GitHub needs a repo.
    pub repo: Option<&'a str>,
}

pub struct PropertyWrite {
    pub display: String,
}

pub struct HoursWrite {
    pub before: f64,
    pub after: f64,
}

pub struct BodyWrite {
    pub blocks_written: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_key_round_trips_through_its_external_id() {
        for key in [
            TaskKey::Notion { page_id: "24f1a2b3c4d56789abcdef0123456789".into() },
            TaskKey::Github {
                host: "github.com".into(),
                owner: "haoov".into(),
                repo: "groove".into(),
                number: 42,
            },
        ] {
            let id = key.external_id();
            assert_eq!(TaskKey::parse(&id).unwrap(), key, "{id}");
        }
    }

    /// A malformed id must say so, not become a Notion page id that 404s.
    #[test]
    fn garbage_is_not_silently_a_notion_page() {
        for bad in ["", "garbage", "gh-groove-1", "github.com/haoov/groove#x"] {
            assert!(TaskKey::parse(bad).is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn the_provider_is_recoverable_from_the_id_alone() {
        assert_eq!(
            TaskKey::parse("24f1a2b3-c4d5-6789-abcd-ef0123456789").unwrap().provider(),
            ProviderId::Notion
        );
        assert_eq!(
            TaskKey::parse("github.com/haoov/groove#3").unwrap().provider(),
            ProviderId::Github
        );
    }
}
