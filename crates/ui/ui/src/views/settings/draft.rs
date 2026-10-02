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

const SHARED: [Ask; 2] = [
    Ask {
        label: "url",
        hint: "git@github.com:team/skills.git",
        secret: false,
    },
    Ask {
        label: "branch",
        hint: "main",
        secret: false,
    },
];

/// What a draft is filling in: a task source, or the team's shared repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drafted {
    Source(ProviderId),
    Shared,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub source: Drafted,
    pub fields: Vec<Field>,
    /// The field the keys go to.
    pub at: Option<usize>,
}

impl Draft {
    /// The source's fields, empty but for GitHub's usual host; the first takes the keys.
    pub fn of(source: ProviderId) -> Self {
        let mut draft = Self::empty(Drafted::Source(source));
        if source == ProviderId::Github {
            draft.fields[0].set("github.com");
        }
        draft
    }

    /// The shared repo's URL and branch, both empty.
    pub fn shared() -> Self {
        Self::empty(Drafted::Shared)
    }

    fn empty(source: Drafted) -> Self {
        Self {
            source,
            fields: asks(source).iter().map(|_| Field::default()).collect(),
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
            Drafted::Source(ProviderId::Notion) => config::Command::ConnectNotion {
                token: Secret::new(text(0)),
                database_id: text(1),
                user_id: text(2),
            },
            Drafted::Source(ProviderId::Github) => config::Command::ConnectGithub { host: text(0) },
            Drafted::Shared => config::Command::JoinShared {
                url: text(0),
                branch: text(1),
            },
        };
        Command::Config(asked)
    }
}

pub fn asks(source: Drafted) -> &'static [Ask] {
    match source {
        Drafted::Source(ProviderId::Notion) => &NOTION,
        Drafted::Source(ProviderId::Github) => &GITHUB,
        Drafted::Shared => &SHARED,
    }
}
