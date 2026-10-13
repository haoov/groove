//! An explorer promoted to a task's session: its worktrees relocated, rows handed over, put back on failure.

use groove_sessions::Moved;
use groove_types::{Error, Session, SessionId, Task, Timestamp, Worktree};

use super::Service;
use crate::task_session;

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
        let relocated = (self.pool)
            .relocate(explorer, &session, worktrees, task.tag())
            .await?;
        let moved: Vec<Moved> = relocated
            .moves()
            .map(|(worktree, branch, path)| Moved {
                worktree: worktree.clone(),
                branch: branch.to_string(),
                path,
            })
            .collect();
        if let Err(e) = self.pool.made_session_dir(&session.id).await {
            relocated.put_back().await;
            return Err(e.into());
        }
        if let Err(e) = self.store.promote(explorer, &session, task, &moved).await {
            relocated.put_back().await;
            return Err(e.into());
        }
        self.pool.prune(explorer).await;
        let worktrees = relocated.landed(&session);
        Ok((session, worktrees))
    }
}
