//! First run: check the machine for missing tools and write the initial config.

use serde::Serialize;

use crate::core::config::{Config, GitConfig, UiConfig};
use crate::core::error::{AppError, AppResult, ErrorKind};
use crate::provider::github::setup::GithubSetup;
use crate::provider::notion::setup::NotionSetup;

/// An external program the app shells out to.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ToolCheck {
    pub name: String,
    /// Resolved path, or None when it is not on PATH.
    pub path: Option<String>,
    /// What stops working without it.
    pub purpose: String,
    /// False = a feature degrades; true = the app cannot work.
    pub required: bool,
    /// Whether a credential-holding tool is logged in; `None` when absent or not applicable.
    pub authed: Option<bool>,
    /// The token's scopes when the CLI reports them. `None` means unknown, not missing.
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Environment {
    /// Where the config file is read from, whether or not it exists.
    pub config_path: String,
    pub config_exists: bool,
    /// Set when the file exists but does not parse.
    pub config_error: Option<String>,
    pub tools: Vec<ToolCheck>,
}

/// Resolve `bin` against the process PATH, after `launch_env::widen_path()`.
fn which(bin: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(bin))
        .find(|candidate| candidate.is_file())
        .map(|p| p.to_string_lossy().to_string())
}

fn check(name: &str, purpose: &str, required: bool) -> ToolCheck {
    ToolCheck {
        name: name.to_string(),
        path: which(name),
        purpose: purpose.to_string(),
        required,
        authed: None,
        scopes: None,
    }
}

/// `<tool> auth status` — the exit code, plus the scopes when it prints them.
async fn forge_authed(tool: &str) -> (Option<bool>, Option<Vec<String>>) {
    if which(tool).is_none() {
        return (None, None);
    }
    let name = tool.to_string();
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&name)
            .args(["auth", "status"])
            .env("NO_COLOR", "1")
            .output()
    })
    .await;

    let Ok(Ok(out)) = out else {
        return (Some(false), None);
    };
    // gh prints to stderr; glab has no scopes line at all.
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (Some(out.status.success()), parse_scopes(&text))
}

/// The scopes out of a `Token scopes: 'a', 'b'` line.
fn parse_scopes(text: &str) -> Option<Vec<String>> {
    let line = text.lines().find(|l| l.contains("Token scopes:"))?;
    let list = line.split("Token scopes:").nth(1)?;
    let scopes: Vec<String> = list
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    (!scopes.is_empty()).then_some(scopes)
}

/// Clipboard tools, one per display server; the app uses the first that fits.
#[cfg(not(target_os = "macos"))]
fn clipboard_tools() -> Vec<ToolCheck> {
    vec![
        check(
            "wl-copy",
            "Copy from the terminal on Wayland (wl-clipboard)",
            false,
        ),
        check("xclip", "Copy from the terminal on X11", false),
        check(
            "xsel",
            "Copy from the terminal on X11 (alternative to xclip)",
            false,
        ),
    ]
}

/// macOS ships `pbcopy`; nothing to report.
#[cfg(target_os = "macos")]
fn clipboard_tools() -> Vec<ToolCheck> {
    vec![]
}

#[cfg(not(target_os = "macos"))]
fn notification_tools() -> Vec<ToolCheck> {
    vec![check(
        "notify-send",
        "Desktop notifications when the window is unfocused",
        false,
    )]
}

/// macOS ships `osascript`; nothing to report.
#[cfg(target_os = "macos")]
fn notification_tools() -> Vec<ToolCheck> {
    vec![]
}

#[tauri::command]
pub async fn check_environment() -> AppResult<Environment> {
    let path = crate::core::config::file_path()
        .ok_or_else(|| AppError::internal("config dir not initialised"))?;
    let exists = path.is_file();
    // A config that fails to parse is not a first run.
    let config_error = if exists {
        crate::core::config::load_config_from_dir(path.parent().unwrap())
            .err()
            .map(|e| e.to_string())
    } else {
        None
    };

    // Resolve `claude` the way the agent spawner does.
    let claude = crate::agent_manager::resolve_claude_bin();
    let claude_path = if claude.contains('/') {
        Some(claude)
    } else {
        which("claude")
    };

    let (glab_auth, gh_auth) =
        futures_util::future::join(forge_authed("glab"), forge_authed("gh")).await;
    let mut glab = check("glab", "GitLab merge requests, threads, CI status", false);
    (glab.authed, glab.scopes) = glab_auth;
    let mut gh = check("gh", "GitHub pull requests, threads, CI status", false);
    (gh.authed, gh.scopes) = gh_auth;

    let mut tools = vec![
        check("git", "Everything: worktrees, diffs, commits", true),
        ToolCheck {
            name: "claude".into(),
            path: claude_path,
            purpose: "The agent console and the MCP tools (Claude Code)".into(),
            required: true,
            authed: None,
            scopes: None,
        },
        // The agent-status hook POSTs with curl.
        check(
            "curl",
            "Agent status (waiting / working / idle) in the dock",
            false,
        ),
        glab,
        gh,
    ];
    tools.extend(notification_tools());
    tools.extend(clipboard_tools());

    Ok(Environment {
        config_path: path.to_string_lossy().to_string(),
        config_exists: exists,
        config_error,
        tools,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_a_tool_from_the_widened_path() {
        crate::launch_env::widen_path();
        assert!(which("sh").is_some(), "sh must resolve");
        assert!(which("definitely-not-a-real-binary-xyz").is_none());
    }

    #[test]
    fn scopes_come_off_the_status_line() {
        let text = "  - Token scopes: 'gist', 'project', 'read:org', 'repo'\n";
        assert_eq!(
            parse_scopes(text).unwrap(),
            ["gist", "project", "read:org", "repo"]
        );
    }

    #[test]
    fn no_scopes_line_means_unknown() {
        assert!(parse_scopes("Logged in to github.com account haoov\n").is_none());
    }

    #[test]
    fn a_missing_tool_is_reported_with_its_purpose() {
        let t = check("definitely-not-a-real-binary-xyz", "Nothing at all", false);
        assert!(t.path.is_none());
        assert!(!t.required);
        assert_eq!(t.purpose, "Nothing at all");
    }
}

/// A shell for the setup screen's interactive CLI sign-in.
#[tauri::command]
pub async fn start_auth_session(
    app: tauri::AppHandle,
    ptys: tauri::State<'_, crate::core::pty::Ptys>,
) -> AppResult<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    crate::agent_manager::start_login_pty(&app, &home, &ptys)
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))
}

/// Write the initial config. Notion property names are detected, then written to the file.
#[tauri::command]
pub async fn write_initial_config(setup: SetupRequest) -> AppResult<()> {
    let root = crate::core::fs::expand_tilde(setup.worktree_root.trim());
    if root.is_empty() {
        return Err(AppError::invalid(
            "A worktree root is required — the directory repos are cloned into.",
        ));
    }
    std::fs::create_dir_all(&root)
        .map_err(|e| AppError::new(ErrorKind::Io, format!("Cannot create {root}: {e}")))?;

    let notion = match &setup.notion {
        Some(n) => Some(crate::provider::notion::setup::build_config(n).await?),
        None => None,
    };
    let github = setup
        .github
        .as_ref()
        .map(|g| crate::provider::github::setup::build_config(g, None));

    let cfg = Config {
        notion,
        github,
        git: GitConfig {
            worktree_root: root,
        },
        ui: UiConfig::default(),
    };
    if !crate::provider::has_task_source(&cfg) {
        return Err(AppError::invalid("Set up at least one task source."));
    }
    crate::core::config::replace(cfg)?;
    Ok(())
}

/// What the setup screen sends: a worktree root plus the sources filled in.
#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct SetupRequest {
    pub worktree_root: String,
    pub notion: Option<NotionSetup>,
    pub github: Option<GithubSetup>,
}

/// Turn one task source on or off after first run. `options` is the provider's
/// setup payload as JSON. Keep the match exhaustive over `ProviderId`.
#[tauri::command]
pub async fn set_task_source(
    provider: crate::provider::types::ProviderId,
    enabled: bool,
    options: serde_json::Value,
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> AppResult<()> {
    use crate::provider::types::ProviderId;

    let mut cfg = crate::core::config::require()?;
    match (provider, enabled) {
        (ProviderId::Notion, true) => {
            let setup: crate::provider::notion::setup::NotionSetup =
                serde_json::from_value(options)
                    .map_err(|e| AppError::invalid(format!("bad Notion setup: {e}")))?;
            cfg.notion = Some(crate::provider::notion::setup::build_config(&setup).await?);
        }
        (ProviderId::Notion, false) => cfg.notion = None,
        (ProviderId::Github, true) => {
            // Reconnecting keeps the names corrected by hand in the config file.
            let setup: crate::provider::github::setup::GithubSetup =
                serde_json::from_value(options)
                    .unwrap_or(crate::provider::github::setup::GithubSetup { host: None });
            cfg.github = Some(crate::provider::github::setup::build_config(
                &setup,
                cfg.github.take(),
            ));
        }
        (ProviderId::Github, false) => cfg.github = None,
    }
    if !crate::provider::has_task_source(&cfg) {
        return Err(AppError::conflict(
            "That would leave no task source at all.",
        ));
    }
    crate::core::config::replace(cfg)?;

    // Prune the disabled source's mirror rows; its sync loop stops. Checked-out tasks stay.
    if !enabled {
        crate::core::db::store::provider_tasks::prune_provider(&*pool, provider.as_str()).await?;
    }
    Ok(())
}
