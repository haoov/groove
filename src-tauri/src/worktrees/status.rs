use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct WorktreeStatus {
    pub worktree_id: String,
    #[ts(type = "number")]
    pub modified: usize,
    #[ts(type = "number")]
    pub staged: usize,
    #[ts(type = "number")]
    pub ahead: i64,
    #[ts(type = "number")]
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

    let (status_out, (ahead, behind)) = tokio::join!(
        crate::core::git::output(&wt.path, &["status", "--porcelain"]),
        ahead_behind(&wt),
    );
    let status_out = status_out.map_err(|e| e.to_string())?;

    let mut modified = 0usize;
    let mut staged = 0usize;
    for change in crate::core::git::porcelain::parse(&String::from_utf8_lossy(&status_out.stdout)) {
        if change.x == '?' && change.y == '?' {
            // An untracked file counts as a working-tree change.
            modified += 1;
        } else {
            if change.x != ' ' {
                staged += 1;
            }
            if change.y != ' ' {
                modified += 1;
            }
        }
    }

    Ok(WorktreeStatus {
        worktree_id,
        modified,
        staged,
        ahead,
        behind,
    })
}

/// Commits `HEAD` leads and trails its upstream by.
async fn ahead_behind(wt: &crate::core::db::models::Worktree) -> (i64, i64) {
    // Ahead/behind against `origin/<branch>`, not the base branch.
    let upstream = format!("origin/{}", wt.branch);
    let has_upstream = crate::core::git::refs::ref_exists(&wt.path, &upstream).await;

    if has_upstream {
        let range = format!("HEAD...{upstream}");
        crate::core::git::output(&wt.path, &["rev-list", "--left-right", "--count", &range])
            .await
            .ok()
            .and_then(|o| {
                if !o.status.success() {
                    return None;
                }
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
        // No upstream: count commits beyond the base ref, honouring a pinned `base_ref`.
        let ahead =
            match crate::core::git::refs::upstream_base(&wt.path, wt.base_ref.as_deref()).await {
                Ok(base_ref) => crate::core::git::output(
                    &wt.path,
                    &["rev-list", "--count", &format!("{base_ref}..HEAD")],
                )
                .await
                .ok()
                .and_then(|o| {
                    if !o.status.success() {
                        return None;
                    }
                    String::from_utf8(o.stdout).ok()?.trim().parse::<i64>().ok()
                })
                .unwrap_or(0),
                Err(_) => 0,
            };
        (ahead, 0)
    }
}
