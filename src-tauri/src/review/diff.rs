use std::collections::HashMap;
use std::time::Duration;

use futures_util::StreamExt;

use super::parse::{parse_unified_diff, unquote_path};
use super::types::{DiffLine, DiffResult, FileDiff, Hunk, RepoDiff};
use crate::core::db::models::Worktree;
use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};
use sqlx::SqlitePool;

/// Minimum interval between fire-and-forget `git fetch origin` calls per repo.
const FETCH_THROTTLE: Duration = Duration::from_secs(60);

/// Untracked files over this size are listed but not rendered.
const UNTRACKED_MAX_BYTES: u64 = 512 * 1024;
/// Untracked files longer than this many lines are truncated in the rendered hunk.
const UNTRACKED_MAX_LINES: usize = 2000;
/// Untracked-file reads in flight at once per worktree.
const UNTRACKED_CONCURRENCY: usize = 6;

/// Args for a diff-producing git call, pinned to the format the parser reads.
/// `-c` is git-level, so the config overrides precede the subcommand.
fn diff_args<'a>(subcommand: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "-c",
        "diff.suppressBlankEmpty=false",
        subcommand,
        "--no-ext-diff",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--submodule=short",
        "--no-color",
        "--no-renames",
    ];
    args.extend_from_slice(rest);
    args
}

/// Args for one commit's diff; `--diff-merges` keeps a merge off the combined format.
fn commit_diff_args<'a>(sha: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut head = vec![sha, "--format=", "--diff-merges=first-parent"];
    head.extend_from_slice(rest);
    diff_args("show", &head)
}

/// A file's text from inside the worktree, refusing an escaping path or an oversized file.
async fn read_worktree_text(root: &str, rel: &str) -> anyhow::Result<String> {
    let full = crate::core::fs::safe_join(root, rel)?;
    let meta = tokio::fs::metadata(&full).await?;
    if meta.len() > UNTRACKED_MAX_BYTES {
        anyhow::bail!(
            "{rel} is {} bytes (cap {UNTRACKED_MAX_BYTES}) — too large to expand",
            meta.len()
        );
    }
    Ok(tokio::fs::read_to_string(&full).await?)
}

/// Kick off a throttled, fire-and-forget `git fetch origin` for a worktree.
fn spawn_throttled_fetch(repo_id: &str, wt_path: &str) {
    if !crate::core::git::cache::shared().due(repo_id, FETCH_THROTTLE) {
        return;
    }
    let fetch_path = wt_path.to_string();
    tokio::spawn(async move {
        if crate::core::git::output(&fetch_path, &["fetch", "origin"])
            .await
            .is_ok()
        {
            crate::core::git::cache::flush();
        }
    });
}

/// Untracked, non-ignored files; `git diff` never lists them.
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
    if let Ok(out) =
        crate::core::git::output(path, &diff_args("diff", &[base_ref, "--name-status"])).await
    {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let Some(st) = parts.next() else { continue };
            let Some(p) = parts.next() else { continue };
            map.insert(
                unquote_path(p),
                st.chars().next().unwrap_or('M').to_string(),
            );
        }
    }
    map
}

/// Map of `path → staged?` from `git status --porcelain`; an untracked file maps
/// to `false`, an absent path has no local change.
async fn staged_map(path: &str) -> HashMap<String, bool> {
    let mut map = HashMap::new();
    if let Ok(out) = crate::core::git::output(path, &["status", "--porcelain"]).await {
        for change in crate::core::git::porcelain::parse(&String::from_utf8_lossy(&out.stdout)) {
            if change.path.is_empty() {
                continue;
            }
            map.insert(
                unquote_path(&change.path),
                change.x != ' ' && change.x != '?',
            );
        }
    }
    map
}

/// Whether a specific path is an untracked (newly created) file.
async fn is_untracked(path: &str, file: &str) -> bool {
    crate::core::git::output(
        path,
        &["ls-files", "--others", "--exclude-standard", "--", file],
    )
    .await
    .ok()
    .filter(|o| o.status.success())
    .map(|o| !o.stdout.is_empty())
    .unwrap_or(false)
}

/// An all-additions hunk for an untracked text file, capped by size and line
/// count; empty for a binary or unreadable file.
async fn untracked_hunks(path: &str, file: &str) -> Vec<Hunk> {
    let Ok(full) = crate::core::fs::safe_join(path, file) else {
        return vec![];
    };

    // An oversized file lists with one placeholder line.
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

    let Ok(content) = tokio::fs::read_to_string(&full).await else {
        return vec![];
    };
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
    read_worktree_text(path, file)
        .await
        .map(|c| c.lines().count() as i64)
        .unwrap_or(0)
}

pub(super) async fn get_task_diff_impl(
    task_id: &str,
    mode: &str,
    pool: &SqlitePool,
) -> anyhow::Result<DiffResult> {
    let worktrees: Vec<Worktree> = store::worktrees::for_session(pool, task_id).await?;

    let mut repo_diffs = vec![];

    for wt in worktrees {
        // "working" mode compares against HEAD; no fetch.
        if mode != "working" {
            spawn_throttled_fetch(&wt.repo_id, &wt.path);
        }

        let base_ref =
            crate::core::git::refs::diff_base(&wt.path, &wt.branch, mode, wt.base_ref.as_deref())
                .await?;

        let diff_output =
            crate::core::git::output(&wt.path, &diff_args("diff", &[&base_ref, "--unified=3"]))
                .await?;
        if !diff_output.status.success() {
            return Err(anyhow::anyhow!(
                "git diff {base_ref} failed: {}",
                String::from_utf8_lossy(&diff_output.stderr).trim()
            ));
        }

        let diff_text = String::from_utf8_lossy(&diff_output.stdout).to_string();
        let mut files = parse_unified_diff(&diff_text);

        // Untracked files render as all-add files.
        for f in list_untracked(&wt.path).await {
            let hunks = untracked_hunks(&wt.path, &f).await;
            let added: i64 = hunks
                .iter()
                .flat_map(|h| h.lines.iter())
                .filter(|l| l.line_type == "add")
                .count() as i64;
            files.push(FileDiff {
                path: f,
                added,
                deleted: 0,
                status: "A".to_string(),
                staged: Some(false),
                hunks,
            });
        }

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

/// Per-file paths and +/- counts only; hunks come lazily from `get_file_diff`.
#[tauri::command]
pub async fn get_task_diff_summary(
    task_id: String,
    mode: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<DiffResult> {
    let mode = mode.as_deref().unwrap_or("vs-main");
    let worktrees: Vec<Worktree> = store::worktrees::for_session(&*pool, &task_id).await?;

    let repos = futures_util::future::try_join_all(
        worktrees.into_iter().map(|wt| summarize_worktree(wt, mode)),
    )
    .await?;

    Ok(DiffResult { task_id, repos })
}

/// One worktree's summary; every read that does not need another's answer runs alongside it.
async fn summarize_worktree(wt: Worktree, mode: &str) -> AppResult<RepoDiff> {
    if mode != "working" {
        spawn_throttled_fetch(&wt.repo_id, &wt.path);
    }

    let base_ref =
        crate::core::git::refs::diff_base(&wt.path, &wt.branch, mode, wt.base_ref.as_deref())
            .await
            .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;

    let numstat_args = diff_args("diff", &[&base_ref, "--numstat"]);
    let (statuses, staged, numstat, untracked) = tokio::join!(
        name_status_map(&wt.path, &base_ref),
        staged_map(&wt.path),
        crate::core::git::output(&wt.path, &numstat_args),
        list_untracked(&wt.path),
    );

    let out = numstat.map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
    if !out.status.success() {
        return Err(AppError::new(
            ErrorKind::Git,
            format!(
                "git diff {base_ref} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
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
            status: statuses
                .get(&path)
                .cloned()
                .unwrap_or_else(|| "M".to_string()),
            staged: staged.get(&path).copied(),
            added: added.parse().unwrap_or(0),
            deleted: deleted.parse().unwrap_or(0),
            path,
            hunks: vec![],
        });
    }

    // Untracked files are not in `git diff`; their line counts read a few at a time.
    let root = wt.path.as_str();
    let mut counted: Vec<(usize, String, i64)> =
        futures_util::stream::iter(untracked.into_iter().enumerate())
            .map(|(at, f)| async move { (at, untracked_added_count(root, &f).await, f) })
            .buffer_unordered(UNTRACKED_CONCURRENCY)
            .map(|(at, added, f)| (at, f, added))
            .collect()
            .await;
    counted.sort_by_key(|(at, _, _)| *at);
    for (_, f, added) in counted {
        files.push(FileDiff {
            path: f,
            added,
            deleted: 0,
            status: "A".to_string(),
            staged: Some(false),
            hunks: vec![],
        });
    }

    Ok(RepoDiff {
        worktree_id: wt.id,
        repo_id: wt.repo_id,
        branch: wt.branch,
        fetch_status: "ok".to_string(),
        files,
    })
}

/// Line content (hunks) for a single file, fetched on demand when displayed.
#[tauri::command]
pub async fn get_file_diff(
    worktree_id: String,
    file_path: String,
    mode: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Vec<Hunk>> {
    let wt = store::worktrees::get(&*pool, &worktree_id).await?;

    let base_ref = crate::core::git::refs::diff_base(
        &wt.path,
        &wt.branch,
        mode.as_deref().unwrap_or("vs-main"),
        wt.base_ref.as_deref(),
    )
    .await
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
    let out = crate::core::git::output(
        &wt.path,
        &diff_args("diff", &[&base_ref, "--unified=3", "--", &file_path]),
    )
    .await
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
    if !out.status.success() {
        return Err(AppError::new(
            ErrorKind::Git,
            format!(
                "git diff {base_ref} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let files = parse_unified_diff(&text);
    let hunks = files
        .into_iter()
        .next()
        .map(|f| f.hunks)
        .unwrap_or_default();
    // An untracked file renders as a new file.
    if hunks.is_empty() && is_untracked(&wt.path, &file_path).await {
        return Ok(untracked_hunks(&wt.path, &file_path).await);
    }
    Ok(hunks)
}

/// Diff of one commit against its parents, hunks included. `git show` handles root commits.
#[tauri::command]
pub async fn get_commit_diff(
    worktree_id: String,
    sha: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Vec<FileDiff>> {
    let wt = store::worktrees::get(&*pool, &worktree_id).await?;

    let out = crate::core::git::output(&wt.path, &commit_diff_args(&sha, &["--unified=3"]))
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
    if !out.status.success() {
        return Err(AppError::new(
            ErrorKind::Git,
            format!(
                "git show {sha} failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }

    // Status letters (A/M/D) per path for the file headers.
    let mut statuses: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(ns) =
        crate::core::git::output(&wt.path, &commit_diff_args(&sha, &["--name-status"])).await
    {
        for line in String::from_utf8_lossy(&ns.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let (Some(st), Some(p)) = (parts.next(), parts.next()) else {
                continue;
            };
            statuses.insert(
                unquote_path(p),
                st.chars().next().unwrap_or('M').to_string(),
            );
        }
    }

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut files = parse_unified_diff(&text);
    for f in &mut files {
        f.added = f
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.line_type == "add")
            .count() as i64;
        f.deleted = f
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.line_type == "del")
            .count() as i64;
        if let Some(st) = statuses.get(&f.path) {
            f.status = st.clone();
        }
        f.staged = None; // a commit has no staging state
    }
    Ok(files)
}

pub async fn get_task_diff_mcp(task_id: &str, pool: &SqlitePool) -> anyhow::Result<DiffResult> {
    get_task_diff_impl(task_id, "vs-main", pool).await
}

// ── Reading file lines (diff context expansion) ────────────────────────────────

/// A slice of a file plus its total line count.
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct FileLines {
    pub lines: Vec<String>,
    pub total: u32,
}

/// Lines `start..=end` (1-indexed, clamped) of a file's new side: the file on
/// disk, or `<rev>:<path>` when `rev` is given.
#[tauri::command]
pub async fn read_file_lines(
    worktree_id: String,
    file_path: String,
    start: u32,
    end: u32,
    rev: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<FileLines> {
    let wt = store::worktrees::get(&*pool, &worktree_id).await?;

    let text = match rev {
        Some(sha) => {
            let out = crate::core::git::output(&wt.path, &["show", &format!("{sha}:{file_path}")])
                .await
                .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
            if !out.status.success() {
                return Err(AppError::new(
                    ErrorKind::Git,
                    format!(
                        "git show {sha}:{file_path} failed: {}",
                        String::from_utf8_lossy(&out.stderr).trim()
                    ),
                ));
            }
            String::from_utf8_lossy(&out.stdout).to_string()
        }
        None => read_worktree_text(&wt.path, &file_path).await?,
    };

    let all: Vec<&str> = text.lines().collect();
    let total = all.len() as u32;
    // Clamp, do not error: a stale gap degrades to an empty slice.
    let from = start.max(1).min(total.saturating_add(1)) as usize;
    let to = end.min(total) as usize;
    let lines = if from > to {
        vec![]
    } else {
        all[from - 1..to].iter().map(|s| s.to_string()).collect()
    };
    Ok(FileLines { lines, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_args_pin_the_output_format() {
        let args = diff_args("diff", &["HEAD", "--unified=3"]);
        assert_eq!(
            &args[..3],
            &["-c", "diff.suppressBlankEmpty=false", "diff"],
            "a -c override must precede the subcommand"
        );
        for flag in [
            "--no-ext-diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--submodule=short",
        ] {
            assert!(args.contains(&flag), "missing {flag} in {args:?}");
        }
        assert_eq!(&args[args.len() - 2..], &["HEAD", "--unified=3"]);
    }

    #[test]
    fn a_merge_commit_diff_follows_the_first_parent() {
        for args in [
            commit_diff_args("deadbeef", &["--unified=3"]),
            commit_diff_args("deadbeef", &["--name-status"]),
        ] {
            assert!(
                args.contains(&"--diff-merges=first-parent"),
                "combined-diff format survives in {args:?}"
            );
            assert!(args.contains(&"--no-ext-diff"));
        }
    }

    #[tokio::test]
    async fn the_untracked_helpers_refuse_a_path_outside_the_worktree() {
        let root = std::env::temp_dir()
            .join("groove-diff-scope")
            .display()
            .to_string();
        for escape in ["/etc/passwd", "../../etc/passwd"] {
            assert!(
                untracked_hunks(&root, escape).await.is_empty(),
                "{escape} rendered"
            );
            assert_eq!(
                untracked_added_count(&root, escape).await,
                0,
                "{escape} counted"
            );
        }
    }

    #[tokio::test]
    async fn reading_worktree_text_refuses_a_path_outside_the_worktree() {
        let root = std::env::temp_dir()
            .join("groove-diff-scope")
            .display()
            .to_string();
        assert!(read_worktree_text(&root, "/etc/passwd").await.is_err());
        assert!(read_worktree_text(&root, "../../etc/passwd").await.is_err());
    }
}
