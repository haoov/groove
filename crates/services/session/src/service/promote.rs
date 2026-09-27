//! An explorer promoted to a task's session: worktrees renamed and moved, rows handed over, undone on failure.

use std::path::{Path, PathBuf};

use groove_git::Git;
use groove_sessions::Moved;
use groove_types::{Error, Session, SessionId, Task, Timestamp, Worktree};
use groove_worktree::naming::default_branch;

use super::Service;
use crate::task_session;

/// One worktree's way from the explorer's directory to the task's.
struct Step {
    worktree: Worktree,
    branch: String,
    path: PathBuf,
}

impl Service {
    /// The task's session, and the worktrees as they stand under it now.
    pub async fn promote(
        &self,
        explorer: &SessionId,
        worktrees: &[Worktree],
        task: &Task,
        now: Timestamp,
    ) -> Result<(Session, Vec<Worktree>), Error> {
        let session = task_session(task, now);
        let steps = self.plan(explorer, &session, worktrees, task);
        let mut done: Vec<&Step> = Vec::new();
        for step in &steps {
            if let Err(e) = walk(step).await {
                undo(&done).await;
                return Err(e);
            }
            done.push(step);
        }
        let moved: Vec<Moved> = steps.iter().map(moved).collect();
        if let Err(e) = self.store.promote(explorer, &session, task, &moved).await {
            undo(&done).await;
            return Err(e.into());
        }
        emptied(&self.pool.layout().session_dir(explorer.as_str()));
        let worktrees = steps.into_iter().map(|one| landed(one, &session)).collect();
        Ok((session, worktrees))
    }

    /// Where each worktree goes: the task's branch for an explorer's own, in the task's directory.
    fn plan(
        &self,
        explorer: &SessionId,
        session: &Session,
        worktrees: &[Worktree],
        task: &Task,
    ) -> Vec<Step> {
        let layout = self.pool.layout();
        let old = layout.session_dir(explorer.as_str());
        let wanted = default_branch(session, Some(task.tag()));
        let mut taken: Vec<(String, String)> = Vec::new();
        worktrees
            .iter()
            .map(|one| {
                let project = project_of(&old, one);
                let branch = match one.branch.starts_with("explorer/") {
                    true => free(&mut taken, one.repo.as_str(), &wanted),
                    false => one.branch.clone(),
                };
                let path = layout.worktree_dir(session.id.as_str(), &project, &branch);
                Step {
                    worktree: one.clone(),
                    branch,
                    path,
                }
            })
            .collect()
    }
}

/// The name the repo does not hold yet: the wanted one, or it with a number after it.
fn free(taken: &mut Vec<(String, String)>, repo: &str, wanted: &str) -> String {
    let used = |name: &str, taken: &[(String, String)]| {
        taken
            .iter()
            .any(|(held, branch)| held == repo && branch == name)
    };
    let mut name = wanted.to_string();
    let mut at = 2;
    while used(&name, taken) {
        name = format!("{wanted}-{at}");
        at += 1;
    }
    taken.push((repo.to_string(), name.clone()));
    name
}

/// The project a worktree stands in: the first directory under the explorer's own.
fn project_of(explorer_dir: &Path, worktree: &Worktree) -> String {
    Path::new(&worktree.path)
        .strip_prefix(explorer_dir)
        .ok()
        .and_then(|rest| rest.iter().next())
        .map(|one| one.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            let repo = worktree.repo.as_str();
            repo.rsplit('/').next().unwrap_or(repo).to_string()
        })
}

/// The branch renamed, then the worktree moved.
async fn walk(step: &Step) -> Result<(), Error> {
    let from = PathBuf::from(&step.worktree.path);
    let git = Git::at(&from);
    if step.branch != step.worktree.branch {
        git.branch_rename(&step.worktree.branch, &step.branch)
            .await?;
    }
    if step.path != from {
        git.worktree_move(&from, &step.path).await?;
    }
    Ok(())
}

/// The steps taken put back, the last first. What cannot be put back is left as it is.
async fn undo(done: &[&Step]) {
    for step in done.iter().rev() {
        let from = PathBuf::from(&step.worktree.path);
        if step.path != from {
            let _ = Git::at(&step.path).worktree_move(&step.path, &from).await;
        }
        if step.branch != step.worktree.branch {
            let _ = Git::at(&from)
                .branch_rename(&step.branch, &step.worktree.branch)
                .await;
        }
    }
}

fn moved(step: &Step) -> Moved {
    Moved {
        worktree: step.worktree.id.clone(),
        branch: step.branch.clone(),
        path: step.path.to_string_lossy().into_owned(),
    }
}

fn landed(step: Step, session: &Session) -> Worktree {
    Worktree {
        session: session.id.clone(),
        branch: step.branch,
        path: step.path.to_string_lossy().into_owned(),
        ..step.worktree
    }
}

/// The explorer's directory taken away when the moves left it holding nothing.
fn emptied(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            emptied(&entry.path());
        }
    }
    let _ = std::fs::remove_dir(dir);
}
