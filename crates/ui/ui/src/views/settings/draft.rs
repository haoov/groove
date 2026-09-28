//! A task source being turned on: the fields it asks for, as they are typed.

use groove_controllers::{Command, config};
use groove_types::{ProviderId, Secret};
use groove_ui_kit::widgets::Field;

/// One field a source asks for: its label, what it shows empty, and whether it is secret.
pub struct Ask {
    pub label: &'static str,
    pub hint: &'static str,
    pub secret: bool,
}

const NOTION: [Ask; 3] = [
    Ask {
        label: "token",
        hint: "the integration's secret",
        secret: true,
    },
    Ask {
        label: "database",
        hint: "the task database's id",
        secret: false,
    },
    Ask {
        label: "user",
        hint: "your Notion user id",
        secret: false,
    },
];

const GITHUB: [Ask; 1] = [Ask {
    label: "host",
    hint: "github.com or your GitHub Enterprise host",
    secret: false,
}];

#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub source: ProviderId,
    pub fields: Vec<Field>,
    /// The field the keys go to.
    pub at: Option<usize>,
}

impl Draft {
    /// The source's fields, empty but for GitHub's usual host; the first takes the keys.
    pub fn of(source: ProviderId) -> Self {
        let mut fields: Vec<Field> = asks(source).iter().map(|_| Field::default()).collect();
        if source == ProviderId::Github {
            fields[0].set("github.com");
        }
        Self {
            source,
            fields,
            at: Some(0),
        }
    }

    /// The field after the one that has the keys, round to the first.
    pub fn next(&mut self) {
        let count = self.fields.len();
        self.at = Some(self.at.map_or(0, |at| (at + 1) % count));
    }

    pub fn focused(&mut self) -> Option<&mut Field> {
        self.fields.get_mut(self.at?)
    }

    /// What connecting asks of the config.
    pub fn command(&self) -> Command {
        let text = |at: usize| self.fields[at].text().trim().to_string();
        let asked = match self.source {
            ProviderId::Notion => config::Command::ConnectNotion {
                token: Secret::new(text(0)),
                database_id: text(1),
                user_id: text(2),
            },
            ProviderId::Github => config::Command::ConnectGithub { host: text(0) },
        };
        Command::Config(asked)
    }
}

pub fn asks(source: ProviderId) -> &'static [Ask] {
    match source {
        ProviderId::Notion => &NOTION,
        ProviderId::Github => &GITHUB,
    }
}
