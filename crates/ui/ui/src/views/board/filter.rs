//! What the board's filter holds, and what it lets through.

use groove_controllers::session_service::Living;
use groove_types::Task;

/// A field a token can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    Status,
    Priority,
    Board,
    Kind,
    Repo,
}

impl Name {
    pub const ALL: [Name; 5] = [
        Name::Status,
        Name::Priority,
        Name::Board,
        Name::Kind,
        Name::Repo,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Name::Status => "status",
            Name::Priority => "priority",
            Name::Board => "board",
            Name::Kind => "kind",
            Name::Repo => "repo",
        }
    }

    fn of(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|name| name.as_str() == text)
    }
}

/// One term: a field with a value, or a bare word the title must hold.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Term {
    Word(String),
    Field(Name, String),
}

/// Every term the filter holds. An item passes when it answers them all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    terms: Vec<Term>,
}

impl Query {
    pub fn of(text: &str) -> Self {
        let terms = text.split_whitespace().map(term).collect();
        Self { terms }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn lets_task(&self, task: &Task) -> bool {
        self.terms.iter().all(|term| match term {
            Term::Word(word) => like(&task.title, word),
            Term::Field(Name::Status, value) => like(&task.status, value),
            Term::Field(Name::Priority, value) => {
                task.priority.is_some_and(|one| like(one.label(), value))
            }
            Term::Field(Name::Board, value) => {
                task.board.as_ref().is_some_and(|one| like(one, value))
            }
            Term::Field(Name::Kind, value) => like("task", value),
            Term::Field(Name::Repo, _) => false,
        })
    }

    pub fn lets_session(&self, living: &Living) -> bool {
        self.terms.iter().all(|term| match term {
            Term::Word(word) => like(&living.session.title, word),
            Term::Field(Name::Kind, value) => like(living.session.kind.name(), value),
            Term::Field(Name::Repo, value) => living
                .worktrees
                .iter()
                .any(|worktree| like(worktree.repo.as_str(), value)),
            Term::Field(_, _) => false,
        })
    }
}

fn term(text: &str) -> Term {
    match text.split_once(':') {
        Some((name, value)) => match Name::of(&name.to_lowercase()) {
            Some(name) => Term::Field(name, value.to_string()),
            None => Term::Word(text.to_string()),
        },
        None => Term::Word(text.to_string()),
    }
}

/// Whether what is held reads as what is wanted: case and word breaks are ignored.
fn like(held: &str, wanted: &str) -> bool {
    loose(held).contains(&loose(wanted))
}

fn loose(text: &str) -> String {
    text.to_lowercase().replace(['-', '_'], " ")
}
