//! Worktrees taken from one session's directory to another's: branches renamed, worktrees moved, undone on failure.

use std::path::{Path, PathBuf};

use groove_git::Git;
use groove_types::{Session, SessionId, Worktree, WorktreeId};

use crate::naming::session_branch;
use crate::{Pool, Result};

/// One worktree's way from the old directory to the new one.
struct Step {
    worktree: Worktree,
    branch: String,
    path: PathBuf,
}

/// The worktrees as the moves left them.
pub struct Relocated {
    steps: Vec<Step>,
}

impl Pool {
    /// Every worktree of `from` under `to`'s directory, an explorer's branch named for `tag`; all put back when one fails.
    pub async fn relocate(
        &self,
        from: &SessionId,
        to: &Session,
        worktrees: &[Worktree],
        tag: &str,
    ) -> Result<Relocated> {
        let relocated = Relocated {
            steps: self.plan(from, to, worktrees, tag).await?,
        };
        for (at, step) in relocated.steps.iter().enumerate() {
            if let Err(e) = walk(step).await {
                undo(&relocated.steps[..at]).await;
                return Err(e);
            }
        }
        Ok(relocated)
    }

    /// `from`'s directory taken away when the moves left it holding nothing.
    pub async fn prune(&self, from: &SessionId) {
        let dir = self.layout.session_dir(from.as_str());
        let _ = tokio::task::spawn_blocking(move || emptied(&dir)).await;
    }

    async fn plan(
        &self,
        from: &SessionId,
        to: &Session,
        worktrees: &[Worktree],
        tag: &str,
    ) -> Result<Vec<Step>> {
        let old = self.layout.session_dir(from.as_str());
        let wanted = session_branch(to, Some(tag));
        let mut taken: Vec<(String, String)> = Vec::new();
        let mut steps = Vec::new();
        for one in worktrees {
            let project = project_of(&old, one);
            let branch = match groove_types::is_explorer_branch(&one.branch) {
                true => free(&mut taken, one, &wanted).await?,
                false => one.branch.clone(),
            };
            let path = self.layout.worktree_dir(to.id.as_str(), &project, &branch);
            steps.push(Step {
                worktree: one.clone(),
                branch,
                path,
            });
        }
        Ok(steps)
    }
}

impl Relocated {
    /// Each worktree with its new branch and path.
    pub fn moves(&self) -> impl Iterator<Item = (&WorktreeId, &str, String)> {
        self.steps.iter().map(|one| {
            let path = one.path.to_string_lossy().into_owned();
            (&one.worktree.id, one.branch.as_str(), path)
        })
    }

    /// Every move put back, the last first.
    pub async fn put_back(&self) {
        undo(&self.steps).await;
    }

    /// The worktrees as `session` now holds them.
    pub fn landed(self, session: &Session) -> Vec<Worktree> {
        let one = |step: Step| Worktree {
            session: session.id.clone(),
            branch: step.branch,
            path: step.path.to_string_lossy().into_owned(),
            ..step.worktree
        };
        self.steps.into_iter().map(one).collect()
    }
}

/// The name the repo does not hold yet: the wanted one, or it with a number after it.
async fn free(
    taken: &mut Vec<(String, String)>,
    worktree: &Worktree,
    wanted: &str,
) -> Result<String> {
    let (repo, git) = (worktree.repo.as_str(), Git::at(&worktree.path));
    let mut name = wanted.to_string();
    let mut at = 2;
    loop {
        let picked = taken
            .iter()
            .any(|(held, branch)| held == repo && *branch == name);
        let held = git.ref_exists(&format!("refs/heads/{name}")).await?;
        if !picked && !held {
            break;
        }
        name = format!("{wanted}-{at}");
        at += 1;
    }
    taken.push((repo.to_string(), name.clone()));
    Ok(name)
}

/// The project a worktree stands in: the first directory under the old session's own.
fn project_of(old: &Path, worktree: &Worktree) -> String {
    Path::new(&worktree.path)
        .strip_prefix(old)
        .ok()
        .and_then(|rest| rest.iter().next())
        .map(|one| one.to_string_lossy().into_owned())
        .unwrap_or_else(|| {
            let repo = worktree.repo.as_str();
            repo.rsplit('/').next().unwrap_or(repo).to_string()
        })
}

/// The branch renamed, then the worktree moved; a move that fails names the branch back.
async fn walk(step: &Step) -> Result<()> {
    let from = PathBuf::from(&step.worktree.path);
    let git = Git::at(&from);
    let renamed = step.branch != step.worktree.branch;
    if renamed {
        git.branch_rename(&step.worktree.branch, &step.branch)
            .await?;
    }
    if step.path != from
        && let Err(e) = git.worktree_move(&from, &step.path).await
    {
        if renamed {
            let _ = git.branch_rename(&step.branch, &step.worktree.branch).await;
        }
        return Err(e.into());
    }
    Ok(())
}

/// The steps taken put back, the last first. What cannot be put back is left as it is.
async fn undo(done: &[Step]) {
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
