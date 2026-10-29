//! An action that asks for its arguments, one prompt at a time.

mod choices;
mod clusters;
mod kinds;

pub use self::kinds::chosen as kind_chosen;

use self::choices::{
    base_prompt, open_choices, pool_choices, repo_choices, session_choices, worktree_choices,
};

use groove_controllers::{AppState, Command, session};
use groove_types::{RepoId, SessionId, WorktreeId, WorktreeSpec};

/// An action that needs arguments before it is a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    AddRepo,
    AddWorktree,
    RemoveRepo,
    SelectWorktree,
    SelectSession,
    CloseWorktree,
    /// Closes a worktree whatever it holds that origin lacks.
    ForceCloseWorktree,
    RenameExplorer,
    ForceDelete,
    AttachCluster,
    DetachCluster,
    /// The kind the Kubernetes list shows.
    PickKind,
}

impl Action {
    /// The id of the command it ends in.
    pub fn id(self) -> &'static str {
        match self {
            Action::AddRepo => "session.add_repo",
            Action::AddWorktree => "session.add_worktree",
            Action::RemoveRepo => "session.remove_repo",
            Action::SelectWorktree => "session.select_worktree",
            Action::SelectSession => "session.select",
            Action::CloseWorktree | Action::ForceCloseWorktree => "session.close_worktree",
            Action::RenameExplorer => "session.rename_explorer",
            Action::ForceDelete => "session.force_delete",
            Action::AttachCluster => "session.attach_cluster",
            Action::DetachCluster => "session.detach_cluster",
            Action::PickKind => "resources.select_kind",
        }
    }
}

/// The picker row that turns *Add repo* into a clone.
pub const CLONE: &str = "\u{0}clone";
const CLONE_LABEL: &str = "Clone from a URL…";

/// One question: `(label, value)` options, `free` for typed text, `allow_empty` for the default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub label: &'static str,
    pub options: Vec<(String, String)>,
    pub free: bool,
    pub allow_empty: bool,
    /// An option with a word that is the whole query comes first.
    pub exact: bool,
}

impl Prompt {
    fn choose(label: &'static str, options: Vec<(String, String)>) -> Self {
        Self {
            label,
            options,
            free: false,
            allow_empty: false,
            exact: false,
        }
    }

    fn text(label: &'static str, allow_empty: bool) -> Self {
        Self {
            label,
            options: Vec::new(),
            free: true,
            allow_empty,
            exact: false,
        }
    }
}

/// An action in progress: what was answered so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub action: Action,
    pub session: SessionId,
    pub answers: Vec<String>,
    /// Each answer as the breadcrumb shows it.
    pub said: Vec<String>,
    /// The entry that started it, and the palette's query then; empty from a chord.
    pub from: (String, String),
}

impl Flow {
    pub fn new(action: Action, session: SessionId) -> Self {
        Self {
            action,
            session,
            answers: Vec::new(),
            said: Vec::new(),
            from: (String::new(), String::new()),
        }
    }

    pub fn answer(&mut self, value: String, said: String) {
        self.answers.push(value);
        self.said.push(said);
    }

    /// The last answer taken back, as it was said.
    pub fn unanswer(&mut self) -> Option<String> {
        self.answers.pop()?;
        self.said.pop()
    }

    /// The next question, or `None` when every answer is in.
    pub fn prompt(&self, app: &AppState) -> Option<Prompt> {
        let sessions = match self.action {
            Action::ForceDelete => Some(session_choices(app)),
            Action::SelectSession => Some(open_choices(app, &self.session)),
            _ => None,
        };
        if let Some(sessions) = sessions {
            let first = self.answers.is_empty();
            return first.then(|| Prompt::choose("session", sessions));
        }
        let open = app.session.get(&self.session)?;
        let step = self.answers.len();
        match (self.action, step) {
            (Action::AddRepo, 0) => Some(Prompt::choose("repo", pool_choices(app, open))),
            (Action::AddRepo, 1) if self.cloning() => Some(Prompt::text("git URL", false)),
            (Action::AddRepo, n) if n == 1 + usize::from(self.cloning()) => {
                Some(Prompt::text("branch", true))
            }
            (Action::AddRepo, n) if n == 2 + usize::from(self.cloning()) => {
                Some(base_prompt(app, &RepoId::new(self.repo_name())))
            }
            (Action::AddWorktree, 0) => Some(Prompt::choose("repo", repo_choices(open))),
            (Action::AddWorktree, 1) => Some(Prompt::text("branch", false)),
            (Action::AddWorktree, 2) => Some(base_prompt(app, &RepoId::new(&self.answers[0]))),
            (Action::RemoveRepo, 0) => Some(Prompt::choose("repo", repo_choices(open))),
            (Action::SelectWorktree | Action::CloseWorktree | Action::ForceCloseWorktree, 0) => {
                Some(Prompt::choose("worktree", worktree_choices(open)))
            }
            (Action::RenameExplorer, 0) => Some(Prompt::text("title", false)),
            (Action::AttachCluster, 0) => Some(Prompt::choose("context", clusters::known(app))),
            (Action::DetachCluster, 0) => Some(Prompt::choose("cluster", clusters::held(open))),
            (Action::PickKind, 0) => Some(Prompt {
                exact: true,
                ..Prompt::choose("kind", kinds::choices(app, open))
            }),
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
                    ..Default::default()
                },
            },
            Action::AddWorktree => session::Command::AddWorktree {
                session,
                repo: RepoId::new(answer(0)),
                spec: WorktreeSpec {
                    branch: non_empty(1),
                    target: non_empty(2),
                    ..Default::default()
                },
            },
            Action::RemoveRepo => session::Command::RemoveRepo {
                session,
                repo: RepoId::new(answer(0)),
                force: false,
            },
            Action::SelectWorktree => select(session, answer(0)),
            Action::SelectSession => session::Command::Select {
                session: SessionId::new(answer(0)),
            },
            Action::CloseWorktree | Action::ForceCloseWorktree => session::Command::CloseWorktree {
                session,
                worktree: WorktreeId::new(answer(0)),
                force: self.action == Action::ForceCloseWorktree,
            },
            Action::RenameExplorer => session::Command::RenameExplorer {
                session,
                title: answer(0),
            },
            Action::ForceDelete => session::Command::ForceDelete {
                session: SessionId::new(answer(0)),
            },
            Action::AttachCluster | Action::DetachCluster => clusters::command(self)?,
            Action::PickKind => return None,
        };
        Some(Command::Session(command))
    }
}

fn select(session: SessionId, worktree: String) -> session::Command {
    let worktree = WorktreeId::new(worktree);
    session::Command::SelectWorktree { session, worktree }
}
