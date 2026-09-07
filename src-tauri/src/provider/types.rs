//! The task shapes every provider speaks in.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertySchema {
    pub name: String,
    /// The shared type vocabulary the frontend renders by.
    pub kind: String,
    /// Allowed values for select / status / multi_select, in the provider's order.
    pub options: Vec<PropertyOption>,
    /// Target database for a relation.
    pub relation_db: Option<String>,
    /// False for formulas, rollups and timestamps: displayable, not settable.
    pub editable: bool,
    /// An id, a timestamp or a computed value; hidden from the property strip.
    pub meta: bool,
}

/// A status property's option groups, as the provider classifies them. Empty
/// when the provider has no grouping.
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
    /// The title property's name.
    pub title_property: String,
    pub properties: Vec<PropertySchema>,
    pub status_groups: Vec<StatusGroup>,
    /// The number field hours are logged to, when this source has one.
    pub hours_property: Option<String>,
}

impl TaskSchema {
    pub fn property(&self, name: &str) -> Option<&PropertySchema> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// The database a relation property points at.
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
        Self {
            id: title.clone(),
            title,
        }
    }
}

/// One property as the panel sees it: kind, current value, and display text.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertyValue {
    pub name: String,
    pub kind: String,
    /// Canonical value. `null` when unset.
    // Keep `unknown`: the default JsonValue binding imports from outside the Vite root.
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

    /// Every provider.
    pub const ALL: [ProviderId; 2] = [ProviderId::Notion, ProviderId::Github];

    /// The stored column back into an id.
    pub fn parse(name: &str) -> anyhow::Result<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.as_str() == name)
            .ok_or_else(|| anyhow::anyhow!("unknown task source {name}"))
    }
}

/// A task's identity at its source. `external_id` is the stored string form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKey {
    Notion {
        page_id: String,
    },
    Github {
        host: String,
        owner: String,
        repo: String,
        number: i64,
    },
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
            TaskKey::Github {
                host,
                owner,
                repo,
                number,
            } => {
                format!("{host}/{owner}/{repo}#{number}")
            }
        }
    }

    /// The stored id back into a key. The provider is a parameter, never inferred
    /// from the id's shape.
    pub fn parse(provider: ProviderId, external_id: &str) -> anyhow::Result<Self> {
        match provider {
            ProviderId::Notion => {
                if !is_uuid(external_id) {
                    anyhow::bail!("not a Notion page id: {external_id}");
                }
                Ok(TaskKey::Notion {
                    page_id: external_id.to_string(),
                })
            }
            ProviderId::Github => {
                let Some((path, number)) = external_id.rsplit_once('#') else {
                    anyhow::bail!("not a GitHub issue id: {external_id}");
                };
                let parts: Vec<&str> = path.split('/').collect();
                let [host, owner, repo] = parts[..] else {
                    anyhow::bail!("not a GitHub issue id: {external_id}");
                };
                let number = number
                    .parse()
                    .map_err(|_| anyhow::anyhow!("not a GitHub issue id: {external_id}"))?;
                Ok(TaskKey::Github {
                    host: host.to_string(),
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                    number,
                })
            }
        }
    }
}

/// Dashed or bare uuid.
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
    /// The board that supplied the fields.
    pub board: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskDraft<'a> {
    pub title: &'a str,
    pub body_markdown: &'a str,
    /// Where to file it, when the provider needs a target.
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
            TaskKey::Notion {
                page_id: "24f1a2b3c4d56789abcdef0123456789".into(),
            },
            TaskKey::Github {
                host: "github.com".into(),
                owner: "haoov".into(),
                repo: "groove".into(),
                number: 42,
            },
        ] {
            let id = key.external_id();
            assert_eq!(TaskKey::parse(key.provider(), &id).unwrap(), key, "{id}");
        }
    }

    #[test]
    fn garbage_does_not_parse() {
        for bad in ["", "garbage", "gh-groove-1"] {
            assert!(
                TaskKey::parse(ProviderId::Notion, bad).is_err(),
                "{bad} as notion"
            );
            assert!(
                TaskKey::parse(ProviderId::Github, bad).is_err(),
                "{bad} as github"
            );
        }
        assert!(TaskKey::parse(ProviderId::Github, "github.com/haoov/groove#x").is_err());
    }

    #[test]
    fn the_id_shape_never_decides_the_provider() {
        let uuid = "24f1a2b3-c4d5-6789-abcd-ef0123456789";
        assert_eq!(
            TaskKey::parse(ProviderId::Notion, uuid).unwrap(),
            TaskKey::Notion {
                page_id: uuid.to_string()
            }
        );
        assert!(TaskKey::parse(ProviderId::Github, uuid).is_err());
    }

    #[test]
    fn a_provider_name_round_trips() {
        for id in ProviderId::ALL {
            assert_eq!(ProviderId::parse(id.as_str()).unwrap(), id);
        }
        assert!(ProviderId::parse("jira").is_err());
    }
}
