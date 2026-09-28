//! Where a new task goes, and what it is filed with there, as the config says.

use groove_types::{Config, ProviderId};

/// The source a new task is filed at, and what it takes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum Filing {
    Notion {
        database_id: String,
        status: Property,
        assignee: Assignee,
    },
    Github {
        host: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Property {
    pub property: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Assignee {
    pub property: String,
    pub user_id: String,
}

/// The source named, or the only one set up.
pub fn filing(config: Option<&Config>, which: Option<ProviderId>) -> Option<Filing> {
    let config = config?;
    match which.or_else(|| only(config))? {
        ProviderId::Notion => {
            let notion = config.notion.as_ref()?;
            let assignee = notion.assignee()?;
            Some(Filing::Notion {
                database_id: notion.database_id.clone(),
                status: Property {
                    property: notion.properties.status.clone(),
                    value: notion.status_map.ready.first().cloned(),
                },
                assignee: Assignee {
                    property: assignee.to_string(),
                    user_id: notion.user_id.clone(),
                },
            })
        }
        ProviderId::Github => Some(Filing::Github {
            host: config.github.as_ref()?.host.clone(),
        }),
    }
}

fn only(config: &Config) -> Option<ProviderId> {
    match (config.notion.is_some(), config.github.is_some()) {
        (true, false) => Some(ProviderId::Notion),
        (false, true) => Some(ProviderId::Github),
        _ => None,
    }
}
