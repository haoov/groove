//! What the board's filter holds, and what it lets through.

use groove_controllers::session_service::Living;
use groove_types::Task;

/// A field a token can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    Status,
    Priority,
    Project,
    Provider,
    Kind,
    Repo,
}

impl Name {
    pub const ALL: [Name; 6] = [
        Name::Status,
        Name::Priority,
        Name::Project,
        Name::Provider,
        Name::Kind,
        Name::Repo,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Name::Status => "status",
            Name::Priority => "priority",
            Name::Project => "project",
            Name::Provider => "provider",
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
            Term::Field(Name::Kind, value) => like("task", value),
            Term::Field(Name::Repo, _) => false,
            Term::Field(name, value) => of_task(*name, value, task),
        })
    }

    /// An MR waiting on the user answers by its title, its author and its project.
    pub fn lets_review(&self, mr: &groove_types::ReviewMr) -> bool {
        self.terms.iter().all(|term| match term {
            Term::Word(word) => like(&mr.title, word) || like(&mr.author, word),
            Term::Field(Name::Kind, value) => like("review", value),
            Term::Field(Name::Repo, value) => like(&mr.project, value),
            Term::Field(Name::Provider, value) => like(mr.forge.as_str(), value),
            Term::Field(..) => false,
        })
    }

    /// A session answers for the task it works, and for what it holds here.
    pub fn lets_session(&self, living: &Living, task: Option<&Task>) -> bool {
        self.terms.iter().all(|term| match term {
            Term::Word(word) => like(&living.session.title, word),
            Term::Field(Name::Kind, value) => like(living.session.kind.name(), value),
            Term::Field(Name::Repo, value) => living
                .worktrees
                .iter()
                .any(|worktree| like(worktree.repo.as_str(), value)),
            Term::Field(name, value) => task.is_some_and(|task| of_task(*name, value, task)),
        })
    }
}

/// What a task holds for one of the fields its own properties answer.
fn of_task(name: Name, value: &str, task: &Task) -> bool {
    match name {
        Name::Status => like(&task.status, value),
        Name::Priority => task.priority.is_some_and(|one| like(one.label(), value)),
        Name::Project => task.project.as_ref().is_some_and(|one| like(one, value)),
        Name::Provider => like(task.provider.as_str(), value),
        Name::Kind | Name::Repo => false,
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
