use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// ─── Module state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct State {
    inner: Arc<StateInner>,
}

struct StateInner {
    // Track the currently visible file for MCP `get_open_file`
    open_file: Mutex<Option<OpenFileState>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                open_file: Mutex::new(None),
            }),
        }
    }

    pub fn get_open_file(&self) -> Option<OpenFileState> {
        self.inner.open_file.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_open_file(&self, state: Option<OpenFileState>) {
        if let Ok(mut g) = self.inner.open_file.lock() {
            *g = state;
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFileState {
    pub task_id: String,
    pub repo_id: String,
    pub file_path: String,
    pub language_id: String,
    pub scroll_top: i64,
    pub cursor_line: i64,
    pub cursor_col: i64,
}

// ─── Search result types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub file: String,
    pub line: u64,
    pub content: String,
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_files(worktree_path: String) -> Result<Vec<String>, String> {
    list_files_impl(&worktree_path).await.map_err(|e| e.to_string())
}

async fn list_files_impl(worktree_path: &str) -> anyhow::Result<Vec<String>> {
    let tracked = crate::core::git::run(worktree_path, &["ls-files"]).await?;
    let untracked =
        crate::core::git::run(worktree_path, &["ls-files", "--others", "--exclude-standard"])
            .await?;

    let mut files: Vec<String> = tracked
        .lines()
        .chain(untracked.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

#[tauri::command]
pub async fn read_file(worktree_path: String, file_path: String) -> Result<String, String> {
    let full = safe_join(&worktree_path, &file_path)?;
    std::fs::read_to_string(&full)
        .map_err(|e| format!("Cannot read {}: {e}", full.display()))
}

#[tauri::command]
pub async fn open_file(
    task_id: String,
    repo_id: String,
    file_path: String,
    language_id: String,
    editor_state: tauri::State<'_, State>,
) -> Result<(), String> {
    editor_state.set_open_file(Some(OpenFileState {
        task_id,
        repo_id,
        file_path,
        language_id,
        scroll_top: 0,
        cursor_line: 0,
        cursor_col: 0,
    }));
    Ok(())
}

#[tauri::command]
pub async fn update_open_file_state(
    scroll_top: Option<i64>,
    cursor_line: Option<i64>,
    cursor_col: Option<i64>,
    editor_state: tauri::State<'_, State>,
) -> Result<(), String> {
    if let Ok(mut guard) = editor_state.inner.open_file.lock() {
        if let Some(ref mut state) = *guard {
            if let Some(s) = scroll_top {
                state.scroll_top = s;
            }
            if let Some(l) = cursor_line {
                state.cursor_line = l;
            }
            if let Some(c) = cursor_col {
                state.cursor_col = c;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn search_files(
    query: String,
    worktree_path: String,
    case_sensitive: Option<bool>,
    max_results: Option<u32>,
) -> Result<Vec<SearchMatch>, String> {
    search_files_impl(&query, &worktree_path, case_sensitive, max_results)
        .await
        .map_err(|e| e.to_string())
}

async fn search_files_impl(
    query: &str,
    path: &str,
    case_sensitive: Option<bool>,
    max_results: Option<u32>,
) -> anyhow::Result<Vec<SearchMatch>> {
    let query = query.to_string();
    let path = path.to_string();
    let case_sensitive = case_sensitive == Some(true);
    let cap = max_results.unwrap_or(200) as usize;

    // Run ripgrep off the async runtime and STREAM its output so we can stop as
    // soon as we've collected `cap` matches (rg's --max-count is per-file, so it
    // can't bound the total; reading the whole output for a common term was the
    // source of the "very slow" feel). Search from the worktree root so the paths
    // come back relative — matching the file tree's ids.
    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<SearchMatch>> {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let mut cmd = Command::new("rg");
        cmd.args(["--json", "--line-number", "--with-filename", "--max-columns", "400", "--max-count", "50"]);
        if !case_sensitive {
            cmd.arg("--ignore-case");
        }
        cmd.arg("--").arg(&query);
        cmd.current_dir(&path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("ripgrep not found: {e}"))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut results = Vec::new();

        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let Ok(item) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
            if item["type"].as_str() != Some("match") {
                continue;
            }
            let data = &item["data"];
            results.push(SearchMatch {
                file: data["path"]["text"].as_str().unwrap_or("").trim_start_matches("./").to_string(),
                line: data["line_number"].as_u64().unwrap_or(0),
                content: data["lines"]["text"].as_str().unwrap_or("").trim_end().to_string(),
            });
            if results.len() >= cap {
                break;
            }
        }

        // Stop rg once we have enough (it may still be scanning a huge tree).
        let _ = child.kill();
        let _ = child.wait();
        Ok(results)
    })
    .await?
}

#[tauri::command]
pub async fn save_file(worktree_path: String, file_path: String, content: String) -> Result<(), String> {
    let full = safe_join(&worktree_path, &file_path)?;
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full, content.as_bytes()).map_err(|e| format!("Cannot write {}: {e}", full.display()))
}

// ─── File tree mutations (create/rename/move/copy/delete) ─────────────────────

/// Join a worktree-relative path safely: rejects absolute paths and any `..`
/// segment so a mutation can never escape the worktree root.
fn safe_join(worktree_path: &str, rel: &str) -> Result<std::path::PathBuf, String> {
    let rel = rel.trim().trim_start_matches('/');
    if rel.is_empty() {
        return Err("empty path".to_string());
    }
    let mut p = std::path::PathBuf::from(worktree_path);
    for comp in std::path::Path::new(rel).components() {
        match comp {
            std::path::Component::Normal(s) => p.push(s),
            std::path::Component::CurDir => {}
            _ => return Err(format!("invalid path: {rel}")),
        }
    }
    Ok(p)
}

#[tauri::command]
pub async fn create_file(worktree_path: String, path: String) -> Result<(), String> {
    let full = safe_join(&worktree_path, &path)?;
    if full.exists() {
        return Err(format!("{path} already exists"));
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full, b"").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_directory(worktree_path: String, path: String) -> Result<(), String> {
    let full = safe_join(&worktree_path, &path)?;
    if full.exists() {
        return Err(format!("{path} already exists"));
    }
    std::fs::create_dir_all(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_path(worktree_path: String, from: String, to: String) -> Result<(), String> {
    let src = safe_join(&worktree_path, &from)?;
    let dst = safe_join(&worktree_path, &to)?;
    if !src.exists() {
        return Err(format!("{from} does not exist"));
    }
    if dst.exists() {
        return Err(format!("{to} already exists"));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&src, &dst).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_path(worktree_path: String, from: String, to: String) -> Result<(), String> {
    let src = safe_join(&worktree_path, &from)?;
    let dst = safe_join(&worktree_path, &to)?;
    if dst.exists() {
        return Err(format!("{to} already exists"));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    copy_recursive(&src, &dst).map_err(|e| e.to_string())
}

fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_path(worktree_path: String, path: String) -> Result<(), String> {
    let full = safe_join(&worktree_path, &path)?;
    let meta = std::fs::symlink_metadata(&full).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&full).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&full).map_err(|e| e.to_string())
    }
}
