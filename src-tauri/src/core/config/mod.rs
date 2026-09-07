use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Absent when Notion is not set up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notion: Option<NotionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<GithubConfig>,
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
    /// Monospace family for the editor, tree and terminal. Must be a fontconfig
    /// family name (`fc-list : family`); an unknown name silently falls through the CSS stack.
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Monospace family for the agent's terminal only; empty = the built-in stack.
    #[serde(default = "default_font_family")]
    pub agent_font_family: String,
    /// One switch for every agent suggestion Groove offers.
    #[serde(default = "default_true")]
    pub suggest_actions: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            theme: default_theme(),
            font_family: default_font_family(),
            agent_font_family: default_font_family(),
            suggest_actions: default_true(),
        }
    }
}

/// Mirrors `DEFAULT_FONT_SIZE` in src/types/ipc.ts.
fn default_font_size() -> u8 {
    15
}

/// Empty = the CSS stack in tokens.css.
fn default_font_family() -> String {
    String::new()
}

fn default_theme() -> String {
    "latte".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionConfig {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub filters: FilterConfig,
    /// Notion page used as the task template when an explorer becomes a task.
    #[serde(default)]
    pub task_template_page_id: Option<String>,
    /// Default Project relation id for tasks created from explorers.
    #[serde(default)]
    pub default_project_id: Option<String>,
}

/// GitHub task source. The token comes from `gh auth token`, not from here.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GithubConfig {
    /// github.com, or a GitHub Enterprise hostname.
    pub host: String,
    pub properties: GithubPropertyNames,
    /// Fallback labels; a write reads the board's own options first.
    pub status_map: StatusMap,
}

/// The board fields the app drives, by name.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GithubPropertyNames {
    pub status: String,
    pub priority: Option<String>,
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

/// The three status values the app writes: filing, picking up and finishing a task.
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
    // The repo pool is discovered on disk under `<worktree_root>/main/**`.
    pub worktree_root: String,
}

/// The config as the frontend sees it: everything except the Notion token.
/// Do not `skip_serializing` the token on `Config` instead; `Config` is also the on-disk format.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ConfigView {
    pub notion: Option<NotionView>,
    pub github: Option<GithubConfig>,
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
            notion: c.notion.map(|n| NotionView {
                database_id: n.database_id,
                user_id: n.user_id,
                properties: n.properties,
                status_map: n.status_map,
                filters: n.filters,
                task_template_page_id: n.task_template_page_id,
                default_project_id: n.default_project_id,
            }),
            github: c.github,
            git: c.git,
            ui: c.ui,
        }
    }
}

pub(crate) const CONFIG_FILE: &str = "workbench.config.json";

// ─── The one process-wide config ──────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Load the config from `config_dir` and remember the directory. Called once at startup.
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

/// Path of the config file.
pub fn file_path() -> Option<PathBuf> {
    CONFIG_DIR.get().map(|dir| dir.join(CONFIG_FILE))
}

/// The config directory.
pub fn dir() -> Option<PathBuf> {
    CONFIG_DIR.get().cloned()
}

/// Mutate the config, persist it and publish it.
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

/// The Notion config, or an error when it is not set up.
pub fn notion() -> anyhow::Result<NotionConfig> {
    require()?
        .notion
        .ok_or_else(|| anyhow::anyhow!("Notion is not set up — add it in Settings"))
}

pub fn github() -> anyhow::Result<GithubConfig> {
    require()?
        .github
        .ok_or_else(|| anyhow::anyhow!("GitHub is not set up — add it in Settings"))
}

pub(crate) fn load_config_from_dir(config_dir: &Path) -> anyhow::Result<Config> {
    let path = config_dir.join(CONFIG_FILE);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Cannot read {}: {e}", path.display()))?;
    Ok(serde_json::from_str(&content)?)
}

/// Write the file atomically (temp + rename) with mode 0600.
pub(crate) fn save_config_to_dir(config_dir: &Path, cfg: &Config) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

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
            github: None,
            notion: Some(NotionConfig {
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
                filters: FilterConfig {
                    exclude_statuses: vec![],
                    filter_by_assignee: true,
                },
                task_template_page_id: None,
                default_project_id: None,
            }),
            git: GitConfig {
                worktree_root: "/tmp/wt".into(),
            },
            ui: UiConfig::default(),
        }
    }

    #[test]
    fn the_view_never_carries_the_token() {
        let json = serde_json::to_string(&ConfigView::from(sample())).unwrap();
        assert!(
            !json.contains("ntn_secret"),
            "token reached the frontend: {json}"
        );
        assert!(
            !json.contains("token"),
            "token field reached the frontend: {json}"
        );
        assert!(json.contains("\"database_id\":\"db\""));
        assert!(json.contains("worktree_root"));
        assert!(json.contains("font_family"));
    }

    #[test]
    fn the_file_on_disk_keeps_the_token() {
        let json = serde_json::to_string(&sample()).unwrap();
        assert!(
            json.contains("ntn_secret"),
            "a saved config without its token cannot authenticate"
        );
    }

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
        let n = cfg.notion.as_ref().expect("notion block");
        assert_eq!(n.status_map.done, "Done");
        assert_eq!(n.properties.assignee.as_deref(), Some("Assignee"));
        assert_eq!(
            n.task_template_page_id.as_deref(),
            Some("c9bff477d2f944fba9846567745a77ec")
        );
        assert_eq!(cfg.ui.font_size, default_font_size());
        assert!(cfg.ui.suggest_actions);
    }

    #[test]
    fn a_ui_block_without_the_new_switch_still_loads() {
        let json = r#"{
          "git": { "worktree_root": "~/worktrees" },
          "ui": { "font_size": 17, "theme": "frappe", "font_family": "Lilex" }
        }"#;
        let cfg: Config = serde_json::from_str(json).expect("must parse");
        assert_eq!(cfg.ui.font_size, 17);
        assert!(cfg.ui.suggest_actions);
    }

    #[test]
    fn the_file_on_disk_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("groove-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        save_config_to_dir(&dir, &sample()).unwrap();
        let mode = std::fs::metadata(dir.join(CONFIG_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "mode was {mode:o}");
        assert!(
            !dir.join(format!("{CONFIG_FILE}.tmp")).exists(),
            "temp file cleaned up"
        );
        let back = load_config_from_dir(&dir).unwrap();
        assert_eq!(back.notion.expect("notion block").token, "ntn_secret");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pre_provider_config_still_loads() {
        let json = r#"{
          "notion": {
            "token": "ntn_x", "database_id": "db", "user_id": "u",
            "properties": { "status": "Status" },
            "status_map": { "ready": "Ready", "in_progress": "Doing", "done": "Done" },
            "filters": { "exclude_statuses": [], "filter_by_assignee": true }
          },
          "git": { "worktree_root": "~/worktrees" }
        }"#;
        let cfg: Config = serde_json::from_str(json).expect("must parse");
        assert!(cfg.notion.is_some());
        assert!(cfg.github.is_none());
    }

    #[test]
    fn a_config_with_no_task_source_loads() {
        let json = r#"{ "git": { "worktree_root": "~/worktrees" } }"#;
        let cfg: Config = serde_json::from_str(json).expect("must parse");
        assert!(cfg.notion.is_none() && cfg.github.is_none());
    }

    #[test]
    fn an_absent_source_is_omitted_on_save() {
        let mut cfg = sample();
        cfg.notion = None;
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("notion"), "{json}");
        assert!(!json.contains("github"), "{json}");
    }

    #[test]
    fn a_saved_config_round_trips() {
        let json = serde_json::to_string(&sample()).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.notion.expect("notion block").token, "ntn_secret");
        assert_eq!(back.ui.font_family, sample().ui.font_family);
    }
}
