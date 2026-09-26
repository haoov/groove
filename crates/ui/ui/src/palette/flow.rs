//! An action that asks for its arguments, one prompt at a time.

use groove_controllers::{AppState, Command, session, session_service::Open};
use groove_types::{RepoId, SessionId, WorktreeId, WorktreeSpec};

/// An action that needs arguments before it is a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AddRepo,
    AddWorktree,
    RemoveRepo,
    SelectWorktree,
    CloseWorktree,
    RenameExplorer,
    ForceDelete,
}

impl Action {
    /// The id of the command it ends in.
    pub fn id(self) -> &'static str {
        match self {
            Action::AddRepo => "session.add_repo",
            Action::AddWorktree => "session.add_worktree",
            Action::RemoveRepo => "session.remove_repo",
            Action::SelectWorktree => "session.select_worktree",
            Action::CloseWorktree => "session.close_worktree",
            Action::RenameExplorer => "session.rename_explorer",
            Action::ForceDelete => "session.force_delete",
        }
    }
}

/// The picker row that turns *Add repo* into a clone.
pub const CLONE: &str = "\u{0}clone";
const CLONE_LABEL: &str = "Clone from a URL…";

/// One question. Options are `(label, value)`; `free` accepts typed text; `allow_empty`
/// lets Enter on nothing mean the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub label: &'static str,
    pub options: Vec<(String, String)>,
    pub free: bool,
    pub allow_empty: bool,
}

impl Prompt {
    fn choose(label: &'static str, options: Vec<(String, String)>) -> Self {
        Self {
            label,
            options,
            free: false,
            allow_empty: false,
        }
    }

    fn text(label: &'static str, allow_empty: bool) -> Self {
        Self {
            label,
            options: Vec::new(),
            free: true,
            allow_empty,
        }
    }
}

/// An action in progress: what was answered so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub action: Action,
    pub session: SessionId,
    pub answers: Vec<String>,
}

impl Flow {
    pub fn new(action: Action, session: SessionId) -> Self {
        Self {
            action,
            session,
            answers: Vec::new(),
        }
    }

    /// The next question, or `None` when every answer is in.
    pub fn prompt(&self, app: &AppState) -> Option<Prompt> {
        if self.action == Action::ForceDelete {
            return self
                .answers
                .is_empty()
                .then(|| Prompt::choose("session", session_choices(app)));
        }
        let open = app.session.get(&self.session)?;
        let step = self.answers.len();
        match (self.action, step) {
            (Action::AddRepo, 0) => Some(Prompt::choose("repo", pool_choices(app, open))),
            (Action::AddRepo, 1) if self.cloning() => Some(Prompt::text("git URL", false)),
            (Action::AddRepo, n) if n == 1 + usize::from(self.cloning()) => {
                Some(Prompt::text("branch, empty for the default", true))
            }
            (Action::AddRepo, n) if n == 2 + usize::from(self.cloning()) => {
                Some(base_prompt(app, &RepoId::new(self.repo_name())))
            }
            (Action::AddWorktree, 0) => Some(Prompt::choose("repo", repo_choices(open))),
            (Action::AddWorktree, 1) => Some(Prompt::text("branch", false)),
            (Action::AddWorktree, 2) => Some(base_prompt(app, &RepoId::new(&self.answers[0]))),
            (Action::RemoveRepo, 0) => Some(Prompt::choose("repo", repo_choices(open))),
            (Action::SelectWorktree | Action::CloseWorktree, 0) => {
                Some(Prompt::choose("worktree", worktree_choices(open)))
            }
            (Action::RenameExplorer, 0) => Some(Prompt::text("title", false)),
            _ => None,
        }
    }

    /// *Add repo* went through the clone row.
    fn cloning(&self) -> bool {
        self.action == Action::AddRepo && self.answers.first().is_some_and(|a| a == CLONE)
    }

    /// The repo *Add repo* names: the picked slug, or the URL to clone.
    fn repo_name(&self) -> &str {
        let at = usize::from(self.cloning());
        self.answers.get(at).map(String::as_str).unwrap_or_default()
    }

    /// The refresh a step needs before its options are current.
    pub fn refresh(&self, app: &AppState) -> Option<Command> {
        let step = self.answers.len();
        match (self.action, step) {
            (Action::AddRepo, 0) => Some(Command::Session(session::Command::ListRepos)),
            (Action::AddRepo, 2) if !self.cloning() => {
                let repo = RepoId::new(self.repo_name());
                let known = app.session.branches.iter().any(|(r, _)| r == &repo);
                (!known).then_some(Command::Session(session::Command::ListBranches { repo }))
            }
            (Action::ForceDelete, 0) => Some(Command::Session(session::Command::List)),
            (Action::AddWorktree, 2) => {
                let repo = RepoId::new(&self.answers[0]);
                let known = app.session.branches.iter().any(|(r, _)| r == &repo);
                (!known).then_some(Command::Session(session::Command::ListBranches { repo }))
            }
            _ => None,
        }
    }

    /// The command, once every answer is in.
    pub fn command(&self) -> Option<Command> {
        let session = self.session.clone();
        let answer = |i: usize| self.answers.get(i).cloned().unwrap_or_default();
        let non_empty = |i: usize| Some(answer(i)).filter(|a| !a.is_empty());
        let shift = usize::from(self.cloning());
        let command = match self.action {
            Action::AddRepo => session::Command::AddRepo {
                session,
                name: self.repo_name().to_string(),
                spec: WorktreeSpec {
                    branch: non_empty(1 + shift),
                    target: non_empty(2 + shift),
                    track_remote: None,
                },
            },
            Action::AddWorktree => session::Command::AddWorktree {
                session,
                repo: RepoId::new(answer(0)),
                spec: WorktreeSpec {
                    branch: non_empty(1),
                    target: non_empty(2),
                    track_remote: None,
                },
            },
            Action::RemoveRepo => session::Command::RemoveRepo {
                session,
                repo: RepoId::new(answer(0)),
                force: false,
            },
            Action::SelectWorktree => session::Command::SelectWorktree {
                session,
                worktree: WorktreeId::new(answer(0)),
            },
            Action::CloseWorktree => session::Command::CloseWorktree {
                session,
                worktree: WorktreeId::new(answer(0)),
                force: false,
            },
            Action::RenameExplorer => session::Command::RenameExplorer {
                session,
                title: answer(0),
            },
            Action::ForceDelete => session::Command::ForceDelete {
                session: SessionId::new(answer(0)),
            },
        };
        Some(Command::Session(command))
    }
}

/// The pool's clones the session does not hold yet, then the clone row.
fn pool_choices(app: &AppState, open: &Open) -> Vec<(String, String)> {
    app.session
        .pool
        .iter()
        .filter(|e| !open.repos.iter().any(|r| r.id.as_str() == e.slug))
        .map(|e| (e.slug.clone(), e.slug.clone()))
        .chain(std::iter::once((
            CLONE_LABEL.to_string(),
            CLONE.to_string(),
        )))
        .collect()
}

/// Every session on disk, open or not.
fn session_choices(app: &AppState) -> Vec<(String, String)> {
    app.session
        .living
        .iter()
        .map(|one| {
            (
                format!("{} · {}", one.session.title, one.session.id),
                one.session.id.to_string(),
            )
        })
        .collect()
}

fn repo_choices(open: &Open) -> Vec<(String, String)> {
    open.repos
        .iter()
        .map(|r| (format!("{} · {}", r.project, r.id), r.id.to_string()))
        .collect()
}

fn worktree_choices(open: &Open) -> Vec<(String, String)> {
    open.worktrees
        .iter()
        .map(|w| {
            let project = open
                .repos
                .iter()
                .find(|r| r.id == w.repo)
                .map(|r| r.project.as_str())
                .unwrap_or("?");
            (format!("{project} · {}", w.branch), w.id.to_string())
        })
        .collect()
}

/// Origin's heads as last listed; empty means the repo's default.
fn base_prompt(app: &AppState, repo: &RepoId) -> Prompt {
    let heads = app
        .session
        .branches
        .iter()
        .find(|(r, _)| r == repo)
        .map(|(_, heads)| heads.iter().map(|h| (h.clone(), h.clone())).collect())
        .unwrap_or_default();
    Prompt {
        label: "base, empty for the repo's default",
        options: heads,
        free: true,
        allow_empty: true,
    }
}
