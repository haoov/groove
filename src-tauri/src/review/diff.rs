use std::collections::HashMap;
use std::time::Duration;

use sqlx::SqlitePool;
use crate::core::db::models::Worktree;
use crate::core::db::store;
use super::parse::{parse_unified_diff, unquote_path};
use super::types::{DiffResult, RepoDiff, FileDiff, Hunk, DiffLine};

/// Fire-and-forget `git fetch origin` at most once a minute per repo: during
/// agent edit bursts the diff view refreshes constantly, and every refresh
/// hitting the network would be waste (worktrees of one repo share a remote).
const FETCH_THROTTLE: Duration = Duration::from_secs(60);

/// Untracked files over this size are listed but not rendered (avoids ballooning
/// the diff payload with a huge all-additions hunk).
const UNTRACKED_MAX_BYTES: u64 = 512 * 1024;
/// Untracked files longer than this many lines are truncated in the rendered hunk.
const UNTRACKED_MAX_LINES: usize = 2000;

/// Kick off a throttled, fire-and-forget `git fetch origin` for a worktree.
/// New remote state means new ref answers, so a completed fetch flushes them.
fn spawn_throttled_fetch(repo_id: &str, wt_path: &str) {
    if !crate::core::git::cache::shared().due(repo_id, FETCH_THROTTLE) {
        return;
    }
    let fetch_path = wt_path.to_string();
    tokio::spawn(async move {
        if crate::core::git::output(&fetch_path, &["fetch", "origin"]).await.is_ok() {
            crate::core::git::cache::flush();
        }
    });
}

/// Untracked, non-ignored files (newly created, not yet `git add`-ed). These never
/// show up in `git diff`, so the diff view has to surface them separately.
async fn list_untracked(path: &str) -> Vec<String> {
    crate::core::git::output(path, &["ls-files", "--others", "--exclude-standard"])
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(unquote_path)
                .collect()
        })
        .unwrap_or_default()
}

/// Map of `path → status letter` ("A"/"M"/"D"/…) from `git diff <base> --name-status`.
async fn name_status_map(path: &str, base_ref: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(out) = crate::core::git::output(
        path,
        &["diff", base_ref, "--name-status", "--no-renames", "--no-color"],
    )
    .await
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let Some(st) = parts.next() else { continue };
            let Some(p) = parts.next() else { continue };
            map.insert(unquote_path(p), st.chars().next().unwrap_or('M').to_string());
        }
    }
    map
}

/// Map of `path → staged?` from `git status --porcelain`: the index side (X) is
/// non-blank and not untracked. Untracked files are present and map to `false`.
/// Paths absent from the map have no local change (committed-only). Rename entries
/// ("R  old -> new") are keyed on the NEW path; quoted paths are unquoted.
async fn staged_map(path: &str) -> HashMap<String, bool> {
    let mut map = HashMap::new();
    if let Ok(out) = crate::core::git::output(path, &["status", "--porcelain"]).await {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if line.len() < 4 {
                continue;
            }
            let x = line.chars().next().unwrap_or(' ');
            let rest = &line[3..];
            // Rename/copy entries read "orig -> new"; key on the destination path.
            let p = rest.rsplit(" -> ").next().unwrap_or(rest);
            map.insert(unquote_path(p), x != ' ' && x != '?');
        }
    }
    map
}

/// Whether a specific path is an untracked (newly created) file.
async fn is_untracked(path: &str, file: &str) -> bool {
    crate::core::git::output(path, &["ls-files", "--others", "--exclude-standard", "--", file])
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Synthesize an all-additions hunk for an untracked text file. Empty for binary or
/// unreadable files (the UI then shows the binary-file note). Large files are capped
/// (by byte size and line count) so they still list without blowing up the payload.
async fn untracked_hunks(path: &str, file: &str) -> Vec<Hunk> {
    let full = std::path::Path::new(path).join(file);

    // Skip reading files that are too large to display; still emit a single
    // placeholder line so the file lists in the diff.
    if let Ok(meta) = tokio::fs::metadata(&full).await {
        if meta.len() > UNTRACKED_MAX_BYTES {
            return vec![Hunk {
                header: "@@ -0,0 +0,0 @@".to_string(),
                lines: vec![DiffLine {
                    num: 0,
                    content: format!("(file too large to display: {} bytes)", meta.len()),
                    line_type: "ctx".to_string(),
                }],
            }];
        }
    }

    let Ok(content) = tokio::fs::read_to_string(&full).await else { return vec![] };
    let total = content.lines().count();
    let mut lines: Vec<DiffLine> = content
        .lines()
        .take(UNTRACKED_MAX_LINES)
        .enumerate()
        .map(|(i, l)| DiffLine {
            num: (i + 1) as i64,
            content: l.to_string(),
            line_type: "add".to_string(),
        })
        .collect();
    if lines.is_empty() {
        return vec![];
    }
    let header = format!("@@ -0,0 +1,{} @@", lines.len());
    if total > UNTRACKED_MAX_LINES {
        lines.push(DiffLine {
            num: 0,
            content: format!("(truncated: showing first {UNTRACKED_MAX_LINES} of {total} lines)"),
            line_type: "ctx".to_string(),
        });
    }
    vec![Hunk { header, lines }]
}

/// Line count of an untracked file for the summary, skipping oversized files.
async fn untracked_added_count(path: &str, file: &str) -> i64 {
    let full = std::path::Path::new(path).join(file);
    match tokio::fs::metadata(&full).await {
        Ok(m) if m.len() > UNTRACKED_MAX_BYTES => 0,
        _ => tokio::fs::read_to_string(&full)
            .await
            .map(|c| c.lines().count() as i64)
            .unwrap_or(0),
    }
}

pub(super) async fn get_task_diff_impl(task_id: &str, mode: &str, pool: &SqlitePool) -> anyhow::Result<DiffResult> {
    let worktrees: Vec<Worktree> = store::worktrees::for_session(pool, task_id).await?;

    let mut repo_diffs = vec![];

    for wt in worktrees {
        // Throttled fire-and-forget fetch so diff is instant; next refresh picks up
        // new remote state. Skip for "working" mode — it compares against HEAD.
        if mode != "working" {
            spawn_throttled_fetch(&wt.repo_id, &wt.path);
        }

        let base_ref = crate::core::git::refs::diff_base(&wt.path, &wt.branch, mode, wt.base_ref.as_deref()).await?;

        // Diff from remote base to the current working tree — captures committed,
        // staged, and unstaged changes in one pass.
        let diff_output = crate::core::git::output(
            &wt.path,
            &["diff", &base_ref, "--unified=3", "--no-color", "--no-renames"],
        )
        .await?;
        if !diff_output.status.success() {
            return Err(anyhow::anyhow!(
                "git diff {base_ref} failed: {}",
                String::from_utf8_lossy(&diff_output.stderr).trim()
            ));
        }

        let diff_text = String::from_utf8_lossy(&diff_output.stdout).to_string();
        let mut files = parse_unified_diff(&diff_text);

        // Untracked (newly created) files: show them as brand-new all-add files.
        for f in list_untracked(&wt.path).await {
            let hunks = untracked_hunks(&wt.path, &f).await;
            let added: i64 = hunks
                .iter()
                .flat_map(|h| h.lines.iter())
                .filter(|l| l.line_type == "add")
                .count() as i64;
            files.push(FileDiff { path: f, added, deleted: 0, status: "A".to_string(), staged: Some(false), hunks });
        }

        // Tag each file with its working-tree staged state (None = committed-only).
        let staged = staged_map(&wt.path).await;
        for f in files.iter_mut() {
            if f.staged.is_none() {
                f.staged = staged.get(&f.path).copied();
            }
        }

        repo_diffs.push(RepoDiff {
            worktree_id: wt.id,
            repo_id: wt.repo_id,
            branch: wt.branch,
            fetch_status: "ok".to_string(),
            files,
        });
    }

    Ok(DiffResult {
        task_id: task_id.to_string(),
        repos: repo_diffs,
    })
}

/// Fast diff summary: per-file paths and +/- counts only, no line content.
/// The UI renders the file tree from this, then fetches each file's hunks lazily
/// via `get_file_diff` — keeps diff mode responsive with many/large files.
#[tauri::command]
pub async fn get_task_diff_summary(
    task_id: String,
    mode: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<DiffResult, String> {
    let mode = mode.as_deref().unwrap_or("vs-main");
    let worktrees: Vec<Worktree> = store::worktrees::for_session(&*pool, &task_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut repo_diffs = vec![];
    for wt in worktrees {
        if mode != "working" {
            spawn_throttled_fetch(&wt.repo_id, &wt.path);
        }

        let base_ref = crate::core::git::refs::diff_base(&wt.path, &wt.branch, mode, wt.base_ref.as_deref())
            .await
            .map_err(|e| e.to_string())?;
        let statuses = name_status_map(&wt.path, &base_ref).await;
        let staged = staged_map(&wt.path).await;
        let out = crate::core::git::output(
            &wt.path,
            &["diff", &base_ref, "--numstat", "--no-renames", "--no-color"],
        )
        .await
        .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "git diff {base_ref} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        let text = String::from_utf8_lossy(&out.stdout);
        let mut files = vec![];
        for line in text.lines() {
            // Format: "<added>\t<deleted>\t<path>"; binary files show "-\t-\t<path>".
            let mut parts = line.splitn(3, '\t');
            let added = parts.next().unwrap_or("0");
            let deleted = parts.next().unwrap_or("0");
            let Some(path) = parts.next() else { continue };
            let path = unquote_path(path);
            files.push(FileDiff {
                status: statuses.get(&path).cloned().unwrap_or_else(|| "M".to_string()),
                staged: staged.get(&path).copied(),
                added: added.parse().unwrap_or(0),
                deleted: deleted.parse().unwrap_or(0),
                path,
                hunks: vec![],
            });
        }

        // Untracked (newly created) files aren't in `git diff` — list them as new files.
        for f in list_untracked(&wt.path).await {
            let added = untracked_added_count(&wt.path, &f).await;
            files.push(FileDiff { path: f, added, deleted: 0, status: "A".to_string(), staged: Some(false), hunks: vec![] });
        }

        repo_diffs.push(RepoDiff {
            worktree_id: wt.id,
            repo_id: wt.repo_id,
            branch: wt.branch,
            fetch_status: "ok".to_string(),
            files,
        });
    }

    Ok(DiffResult { task_id, repos: repo_diffs })
}

/// Line content (hunks) for a single file, fetched on demand when displayed.
#[tauri::command]
pub async fn get_file_diff(
    worktree_id: String,
    file_path: String,
    mode: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Hunk>, String> {
    let wt = store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;

    let base_ref = crate::core::git::refs::diff_base(
        &wt.path,
        &wt.branch,
        mode.as_deref().unwrap_or("vs-main"),
        wt.base_ref.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let out = crate::core::git::output(
        &wt.path,
        &["diff", &base_ref, "--unified=3", "--no-color", "--no-renames", "--", &file_path],
    )
    .await
    .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "git diff {base_ref} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let files = parse_unified_diff(&text);
    let hunks = files.into_iter().next().map(|f| f.hunks).unwrap_or_default();
    // No tracked diff but the file is untracked → render it as a brand-new file.
    if hunks.is_empty() && is_untracked(&wt.path, &file_path).await {
        return Ok(untracked_hunks(&wt.path, &file_path).await);
    }
    Ok(hunks)
}

/// Diff of one commit against its parent(s), fully populated (hunks included) —
/// commits are immutable, so one eager payload beats the lazy per-file plumbing
/// the working-tree views need. `git show` handles root commits.
#[tauri::command]
pub async fn get_commit_diff(
    worktree_id: String,
    sha: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<FileDiff>, String> {
    let wt = store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;

    let out = crate::core::git::output(
        &wt.path,
        &["show", &sha, "--format=", "--unified=3", "--no-color", "--no-renames"],
    )
    .await
    .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "git show {sha} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // Status letters (A/M/D) per path for the file headers.
    let mut statuses: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(ns) = crate::core::git::output(
        &wt.path,
        &["show", &sha, "--format=", "--name-status", "--no-renames"],
    )
    .await
    {
        for line in String::from_utf8_lossy(&ns.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let (Some(st), Some(p)) = (parts.next(), parts.next()) else { continue };
            statuses.insert(unquote_path(p), st.chars().next().unwrap_or('M').to_string());
        }
    }

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut files = parse_unified_diff(&text);
    for f in &mut files {
        f.added = f.hunks.iter().flat_map(|h| &h.lines).filter(|l| l.line_type == "add").count() as i64;
        f.deleted = f.hunks.iter().flat_map(|h| &h.lines).filter(|l| l.line_type == "del").count() as i64;
        if let Some(st) = statuses.get(&f.path) {
            f.status = st.clone();
        }
        f.staged = None; // historical snapshot — staging doesn't apply
    }
    Ok(files)
}

pub async fn get_task_diff_mcp(task_id: &str, pool: &SqlitePool) -> anyhow::Result<DiffResult> {
    get_task_diff_impl(task_id, "vs-main", pool).await
}

// ── Reading file lines (diff context expansion) ────────────────────────────────

/// A slice of a file, plus how long the file is so a caller can tell whether a
/// trailing gap still has more to show.
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct FileLines {
    pub lines: Vec<String>,
    pub total: u32,
}

/// Lines `start..=end` (1-indexed, inclusive, clamped) of a file's NEW side.
///
/// `rev` is None for the three working-tree diff modes: `get_file_diff` runs
/// `git diff <base>` with no second rev, so the "after" content is the file on disk.
/// A commit diff is `git show <sha>`, whose new side is that commit — those callers
/// pass the sha and the content comes from git instead.
#[tauri::command]
pub async fn read_file_lines(
    worktree_id: String,
    file_path: String,
    start: u32,
    end: u32,
    rev: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<FileLines, String> {
    let wt = store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;

    let text = match rev {
        Some(sha) => {
            let out = crate::core::git::output(&wt.path, &["show", &format!("{sha}:{file_path}")])
                .await
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err(format!(
                    "git show {sha}:{file_path} failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        }
        None => {
            let full = std::path::Path::new(&wt.path).join(&file_path);
            // Same cap the untracked-file path uses: never pull a huge blob into the
            // webview just because someone expanded a gap.
            let meta = tokio::fs::metadata(&full).await.map_err(|e| e.to_string())?;
            if meta.len() > UNTRACKED_MAX_BYTES {
                return Err(format!(
                    "{file_path} is {} bytes (cap {UNTRACKED_MAX_BYTES}) — too large to expand",
                    meta.len()
                ));
            }
            tokio::fs::read_to_string(&full).await.map_err(|e| e.to_string())?
        }
    };

    let all: Vec<&str> = text.lines().collect();
    let total = all.len() as u32;
    // Clamp rather than error: a gap computed against a stale hunk list must degrade
    // to "nothing more to show", not fail the click.
    let from = start.max(1).min(total.saturating_add(1)) as usize;
    let to = end.min(total) as usize;
    let lines = if from > to {
        vec![]
    } else {
        all[from - 1..to].iter().map(|s| s.to_string()).collect()
    };
    Ok(FileLines { lines, total })
}
