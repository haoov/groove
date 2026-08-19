use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub worktree_id: String,
    pub modified: usize,
    pub staged: usize,
    pub ahead: i64,
    pub behind: i64,
}

#[tauri::command]
pub async fn get_worktree_status(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<WorktreeStatus, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;

    let status_out = super::run_git_output(&wt.path, &["status", "--porcelain"])
        .await
        .map_err(|e| e.to_string())?;

    let mut modified = 0usize;
    let mut staged = 0usize;
    for line in String::from_utf8_lossy(&status_out.stdout).lines() {
        if line.len() >= 2 {
            let x = line.chars().next().unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            if x == '?' && y == '?' {
                // Untracked (newly created) file — count it as a working-tree change.
                modified += 1;
            } else {
                if x != ' ' { staged += 1; }
                if y != ' ' { modified += 1; }
            }
        }
    }

    // Ahead/behind vs the branch's own remote tracking ref (origin/<branch>),
    // not vs main — that distance powers the diff view instead.
    let upstream = format!("origin/{}", wt.branch);
    let has_upstream = super::run_git_output(&wt.path, &["rev-parse", "--verify", &upstream])
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let (ahead, behind) = if has_upstream {
        let range = format!("HEAD...{upstream}");
        super::run_git_output(&wt.path, &["rev-list", "--left-right", "--count", &range])
            .await
            .ok()
            .and_then(|o| {
                if !o.status.success() { return None; }
                let s = String::from_utf8(o.stdout).ok()?;
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() == 2 {
                    Some((parts[0].parse::<i64>().ok()?, parts[1].parse::<i64>().ok()?))
                } else {
                    None
                }
            })
            .unwrap_or((0, 0))
    } else {
        // Never pushed: every commit beyond the base is unpushed. Honours a review
        // worktree's pinned target, so this count agrees with the diff instead of
        // measuring against the repo default. No base on origin means there is
        // nothing to measure against, and the count stays 0.
        let ahead = match super::upstream_base(&wt.path, wt.base_ref.as_deref()).await {
            Ok(base_ref) => super::run_git_output(&wt.path, &["rev-list", "--count", &format!("{base_ref}..HEAD")])
                .await
                .ok()
                .and_then(|o| {
                    if !o.status.success() { return None; }
                    String::from_utf8(o.stdout).ok()?.trim().parse::<i64>().ok()
                })
                .unwrap_or(0),
            Err(_) => 0,
        };
        (ahead, 0)
    };

    Ok(WorktreeStatus { worktree_id, modified, staged, ahead, behind })
}
