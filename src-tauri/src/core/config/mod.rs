use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub notion: NotionConfig,
    pub git: GitConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct UiConfig {
    #[serde(default = "default_font_size")]
    pub font_size: u8,
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Monospace family for the editor, tree and terminal. Must be a name
    /// fontconfig reports (`fc-list : family`) — a name nothing matches falls
    /// silently through to the next family in the CSS stack.
    #[serde(default = "default_font_family")]
    pub font_family: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            theme: default_theme(),
            font_family: default_font_family(),
        }
    }
}

/// Mirrors `DEFAULT_FONT_SIZE` in src/types/ipc.ts.
fn default_font_size() -> u8 {
    15
}

/// Empty = use the CSS stack in tokens.css, which tries the Nerd Font variants and
/// falls back to `ui-monospace`. A name written here on a machine that lacks the
/// font is a silent fallback with no way to tell why; no name is honest.
fn default_font_family() -> String {
    String::new()
}

fn default_theme() -> String {
    "frappe".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionConfig {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub filters: FilterConfig,
    /// Notion page whose body is mirrored as the template when an explorer
    /// session is converted into a task. Required for that conversion.
    #[serde(default)]
    pub task_template_page_id: Option<String>,
    /// Optional default Project relation id set on tasks created from explorers.
    #[serde(default)]
    pub default_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PropertyNames {
    pub status: String,
    pub priority: Option<String>,
    pub sprint: Option<String>,
    pub project: Option<String>,
    pub assignee: Option<String>,
}

/// The three status values the app WRITES: filing a task, picking it up, finishing
/// it. Detected from the database (see `detect.rs`) and kept here so a wrong guess
/// can be corrected.
///
/// There is deliberately no `blocked` or `in_review`: nothing ever set them, and the
/// database this was written against has no "In review" option at all — the map was
/// describing states the app does not drive. Reading a status is a different problem
/// and needs no map: `lib/taskStatus.ts` classifies whatever label Notion returns.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct StatusMap {
    pub ready: String,
    pub in_progress: String,
    pub done: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct FilterConfig {
    pub exclude_statuses: Vec<String>,
    #[serde(default = "default_true")]
    pub filter_by_assignee: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GitConfig {
    // The repo pool is discovered on disk (`<worktree_root>/main/**`), not
    // configured — an old `repos` key in the file is silently ignored.
    pub worktree_root: String,
}

/// The config as the FRONTEND sees it: everything except the Notion token.
///
/// The token is a write credential for the whole task database, and nothing in the
/// webview uses it — every Notion call is made in Rust. It cannot just be
/// `skip_serializing` on the field, because `Config` is also what gets written back
/// to `workbench.config.json` (see `save_config_to_dir`), so skipping it there would
/// erase the token from disk on the next preference change.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ConfigView {
    pub notion: NotionView,
    pub git: GitConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct NotionView {
    pub database_id: String,
    pub user_id: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub filters: FilterConfig,
    pub task_template_page_id: Option<String>,
    pub default_project_id: Option<String>,
}

impl From<Config> for ConfigView {
    fn from(c: Config) -> Self {
        Self {
            notion: NotionView {
                database_id: c.notion.database_id,
                user_id: c.notion.user_id,
                properties: c.notion.properties,
                status_map: c.notion.status_map,
                filters: c.notion.filters,
                task_template_page_id: c.notion.task_template_page_id,
                default_project_id: c.notion.default_project_id,
            },
            git: c.git,
            ui: c.ui,
        }
    }
}

/// Named once: the setup flow reports this path to the user.
pub(crate) const CONFIG_FILE: &str = "workbench.config.json";

// ─── The one process-wide config ──────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Remember where the config lives and load it if it exists. Called once at
/// startup, before anything reads it; a missing file is the first-run state,
/// a corrupt one is reported by `check_environment`.
pub fn init(config_dir: PathBuf) {
    match load_config_from_dir(&config_dir) {
        Ok(cfg) => set(cfg),
        Err(e) => tracing::warn!("config not loaded: {e}"),
    }
    let _ = CONFIG_DIR.set(config_dir);
}

pub fn get() -> Option<Config> {
    CONFIG.read().ok().and_then(|g| g.clone())
}

pub fn require() -> anyhow::Result<Config> {
    get().ok_or_else(|| anyhow::anyhow!("not configured — run the setup screen first"))
}

/// Where the config file lives (shown by the setup screen, read by env checks).
pub fn file_path() -> Option<PathBuf> {
    CONFIG_DIR.get().map(|dir| dir.join(CONFIG_FILE))
}

/// Mutate the config, persist it, and publish it — one write path for setup
/// and every preference change.
pub fn update(edit: impl FnOnce(&mut Config)) -> anyhow::Result<Config> {
    let mut cfg = require()?;
    edit(&mut cfg);
    persist(&cfg)?;
    set(cfg.clone());
    Ok(cfg)
}

/// Install a complete config (first-run setup) and persist it.
pub fn replace(cfg: Config) -> anyhow::Result<()> {
    persist(&cfg)?;
    set(cfg);
    Ok(())
}

fn set(cfg: Config) {
    if let Ok(mut guard) = CONFIG.write() {
        *guard = Some(cfg);
    }
}

fn persist(cfg: &Config) -> anyhow::Result<()> {
    let dir = CONFIG_DIR
        .get()
        .ok_or_else(|| anyhow::anyhow!("config dir not initialised"))?;
    std::fs::create_dir_all(dir)?;
    save_config_to_dir(dir, cfg)
}

pub(crate) fn load_config_from_dir(config_dir: &Path) -> anyhow::Result<Config> {
    let path = config_dir.join(CONFIG_FILE);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Cannot read {}: {e}", path.display()))?;
    Ok(serde_json::from_str(&content)?)
}

/// Write the file atomically (temp + rename, so a crash never truncates it)
/// and owner-only (0600) — it holds the Notion token.
pub(crate) fn save_config_to_dir(config_dir: &Path, cfg: &Config) -> anyhow::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::io::Write;

    let path = config_dir.join(CONFIG_FILE);
    let tmp = config_dir.join(format!("{CONFIG_FILE}.tmp"));
    let content = serde_json::to_string_pretty(cfg)?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);

    std::fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Config {
        Config {
            notion: NotionConfig {
                token: "ntn_secret".into(),
                database_id: "db".into(),
                user_id: "user".into(),
                properties: PropertyNames {
                    status: "Status".into(),
                    priority: None,
                    sprint: None,
                    project: None,
                    assignee: None,
                },
                status_map: StatusMap {
                    ready: "Ready".into(),
                    in_progress: "In progress".into(),
                    done: "Done".into(),
                },
                filters: FilterConfig { exclude_statuses: vec![], filter_by_assignee: true },
                task_template_page_id: None,
                default_project_id: None,
            },
            git: GitConfig { worktree_root: "/tmp/wt".into() },
            ui: UiConfig::default(),
        }
    }

    #[test]
    fn the_view_never_carries_the_token() {
        let json = serde_json::to_string(&ConfigView::from(sample())).unwrap();
        assert!(!json.contains("ntn_secret"), "token reached the frontend: {json}");
        assert!(!json.contains("token"), "token field reached the frontend: {json}");
        // Everything the frontend does need is still there.
        assert!(json.contains("\"database_id\":\"db\""));
        assert!(json.contains("worktree_root"));
        assert!(json.contains("font_family"));
    }

    #[test]
    fn the_file_on_disk_keeps_the_token() {
        let json = serde_json::to_string(&sample()).unwrap();
        assert!(json.contains("ntn_secret"), "a saved config without its token cannot authenticate");
    }

    /// A config written before `blocked`/`in_review` were dropped must still load:
    /// serde ignores unknown keys, so an existing install keeps working untouched.
    #[test]
    fn an_older_config_with_removed_keys_still_loads() {
        let json = r#"{
          "notion": {
            "token": "ntn_x", "database_id": "db", "user_id": "u",
            "properties": { "status": "Status", "priority": "Priority", "sprint": "Sprint",
                            "project": "Project", "assignee": "Assignee" },
            "status_map": { "ready": "Ready for sprint", "in_progress": "In progress",
                            "blocked": "Blocked", "in_review": "In review", "done": "Done" },
            "filters": { "exclude_statuses": ["Done"], "filter_by_assignee": true },
            "task_template_page_id": "c9bff477d2f944fba9846567745a77ec"
          },
          "git": { "worktree_root": "~/worktrees" }
        }"#;
        let cfg: Config = serde_json::from_str(json).expect("an existing config must still parse");
        assert_eq!(cfg.notion.status_map.done, "Done");
        assert_eq!(cfg.notion.properties.assignee.as_deref(), Some("Assignee"));
        assert_eq!(cfg.notion.task_template_page_id.as_deref(), Some("c9bff477d2f944fba9846567745a77ec"));
        // `ui` is absent in older files and must default rather than fail.
        assert_eq!(cfg.ui.font_size, default_font_size());
    }

    /// The file holds the Notion token, so it must be owner-only — and written
    /// via a temp+rename so a crash mid-save can never truncate it.
    #[test]
    fn the_file_on_disk_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("groove-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        save_config_to_dir(&dir, &sample()).unwrap();
        let mode = std::fs::metadata(dir.join(CONFIG_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "mode was {mode:o}");
        assert!(!dir.join(format!("{CONFIG_FILE}.tmp")).exists(), "temp file cleaned up");
        let back = load_config_from_dir(&dir).unwrap();
        assert_eq!(back.notion.token, "ntn_secret");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_config_round_trips() {
        let json = serde_json::to_string(&sample()).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.notion.token, "ntn_secret");
        assert_eq!(back.ui.font_family, sample().ui.font_family);
    }
}
