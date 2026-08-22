//! First run: what the machine is missing, and writing the config once.
//!
//! Everything the app needs beyond itself is checked here and reported in one
//! payload, so a new user sees the whole list at once instead of discovering each
//! missing tool through a failed action.

use serde::Serialize;

use crate::core::config::{Config, FilterConfig, GitConfig, NotionConfig, UiConfig};

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
    /// For tools that hold their own credentials: whether they are logged in.
    /// `None` when the tool is absent or has nothing to authenticate.
    ///
    /// Installed-but-not-logged-in is the state worth naming: every MR feature
    /// fails, and the CLI's own error ("not logged in") only appears once you try.
    pub authed: Option<bool>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Environment {
    /// Where the config file is read from, whether or not it exists.
    pub config_path: String,
    pub config_exists: bool,
    /// Set when the file exists but could not be parsed — a missing key names itself.
    pub config_error: Option<String>,
    pub tools: Vec<ToolCheck>,
}

/// Resolve against the process PATH — which `launch_env::widen_path()` has already
/// extended, so a tool installed by linuxbrew or npm is found even though a desktop
/// launch never sourced the shell profile that would have added it.
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
    }
}

/// `<tool> auth status` — exit code only, which is all the CLIs promise.
async fn forge_authed(tool: &str) -> Option<bool> {
    which(tool)?;
    let tool = tool.to_string();
    let ok = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&tool)
            .args(["auth", "status"])
            .env("NO_COLOR", "1")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);
    Some(ok)
}

/// One clipboard tool is enough, so they are reported as a group: the app writes to
/// every one it finds (Wayland and X11 clipboards are separate).
fn clipboard_tools() -> Vec<ToolCheck> {
    vec![
        check("wl-copy", "Copy from the terminal on Wayland (wl-clipboard)", false),
        check("xclip", "Copy from the terminal on X11", false),
        check("xsel", "Copy from the terminal on X11 (alternative to xclip)", false),
    ]
}

#[tauri::command]
pub async fn check_environment() -> Result<Environment, String> {
    let path = crate::core::config::file_path()
        .ok_or_else(|| "config dir not initialised".to_string())?;
    let exists = path.is_file();
    // Distinguish "not set up yet" from "set up wrongly": a config that fails to
    // parse must not look like a first run, or the fix is invisible.
    let config_error = if exists {
        crate::core::config::load_config_from_dir(path.parent().unwrap())
            .err()
            .map(|e| e.to_string())
    } else {
        None
    };

    // `claude` is resolved the way the agent spawner resolves it, not by PATH
    // alone: an npm install under ~/.local/bin works even when PATH omits it, and
    // reporting it as missing there would send the user chasing nothing.
    let claude = crate::agent_manager::resolve_claude_bin();
    let claude_path = if claude.contains('/') { Some(claude) } else { which("claude") };

    // Both auth checks at once: each shells out, and a serial pair is a visible
    // pause on a screen whose whole job is to answer "is this machine ready?".
    let (glab_auth, gh_auth) = futures_util::future::join(forge_authed("glab"), forge_authed("gh")).await;
    let mut glab = check("glab", "GitLab merge requests, threads, CI status", false);
    glab.authed = glab_auth;
    let mut gh = check("gh", "GitHub pull requests, threads, CI status", false);
    gh.authed = gh_auth;

    let mut tools = vec![
        check("git", "Everything: worktrees, diffs, commits", true),
        ToolCheck {
            name: "claude".into(),
            path: claude_path,
            purpose: "The agent console and the MCP tools (Claude Code)".into(),
            required: true,
            authed: None,
        },
        // Claude Code reports what the agent is doing by POSTing to the app from a
        // hook, and the hook command is a curl. Without it the agent still works,
        // but the dock and the console never say what it is doing.
        check("curl", "Agent status (waiting / working / idle) in the dock", false),
        // One CLI per forge, each owning its own auth. Only the forges you actually
        // have repos on matter, so both are optional.
        glab,
        gh,
        check("notify-send", "Desktop notifications when the window is unfocused", false),
    ];
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
        // `sh` is on every PATH; this is really checking `which` itself works.
        crate::launch_env::widen_path();
        assert!(which("sh").is_some(), "sh must resolve");
        assert!(which("definitely-not-a-real-binary-xyz").is_none());
    }

    #[test]
    fn a_missing_tool_is_reported_with_its_purpose() {
        let t = check("definitely-not-a-real-binary-xyz", "Nothing at all", false);
        assert!(t.path.is_none());
        assert!(!t.required);
        assert_eq!(t.purpose, "Nothing at all");
    }
}

/// A shell for the setup screen's sign-in.
///
/// Both CLIs authenticate interactively — a device code to copy, a browser to open, a
/// token to paste — so there is nothing to automate here. The user gets a real shell
/// rather than the login command itself, because the command is not always the same
/// one: a self-hosted GitLab needs `glab auth login --hostname <host>`.
#[tauri::command]
pub async fn start_auth_session(
    app: tauri::AppHandle,
    ptys: tauri::State<'_, crate::core::pty::Ptys>,
) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    crate::agent_manager::start_login_pty(&app, &home, &ptys).map_err(|e| e.to_string())
}

/// What the database says about itself, for the setup screen to show before saving.
///
/// The point is that the user can SEE what was detected: a silent wrong guess about
/// which property holds the status is worse than a visible one.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct DetectedSchema {
    pub title_property: String,
    pub status_property: String,
    pub priority_property: Option<String>,
    pub sprint_property: Option<String>,
    pub project_property: Option<String>,
    pub assignee_property: Option<String>,
    /// The status values the app will write when filing / starting / finishing.
    pub status_ready: String,
    pub status_in_progress: String,
    pub status_done: String,
    /// Every status option, so a wrong pick is obvious in context.
    pub status_options: Vec<String>,
}

/// Read the database's vocabulary. Also the check that the integration can see it.
#[tauri::command]
pub async fn detect_database(token: String, database_id: String) -> Result<DetectedSchema, String> {
    let schema = crate::notion::schema::load(&token, database_id.trim())
        .await
        .map_err(|e| format!("Cannot read that database: {e}"))?;
    let props = crate::notion::detect::detect_properties(&schema);
    let status = crate::notion::detect::detect_status_map(&schema);
    Ok(DetectedSchema {
        title_property: schema.title_property.clone(),
        status_property: props.status.clone(),
        priority_property: props.priority.clone(),
        sprint_property: props.sprint.clone(),
        project_property: props.project.clone(),
        assignee_property: props.assignee.clone(),
        status_ready: status.ready,
        status_in_progress: status.in_progress,
        status_done: status.done,
        status_options: schema
            .properties
            .iter()
            .find(|p| p.name == props.status)
            .map(|p| p.options.clone())
            .unwrap_or_default(),
    })
}

/// Write the initial config.
///
/// Property names and status values are DETECTED from the database rather than
/// asked for or defaulted: Notion already knows which property is the status and
/// which of its options mean to-do / in-progress / complete. They are still written
/// to the file, so a wrong detection can be corrected without a rebuild.
#[tauri::command]
pub async fn write_initial_config(
    token: String,
    database_id: String,
    user_id: String,
    worktree_root: String,
    template_page_id: Option<String>,
) -> Result<(), String> {
    if token.trim().is_empty() || database_id.trim().is_empty() {
        return Err("A Notion token and database id are both required.".into());
    }
    let root = crate::core::fs::expand_tilde(worktree_root.trim());
    if root.is_empty() {
        return Err("A worktree root is required — the directory repos are cloned into.".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| format!("Cannot create {root}: {e}"))?;

    // Reading the schema is both the detection and the check that the integration
    // can see this database — the most likely mistake, and one that would otherwise
    // surface later as an empty task list.
    let schema = crate::notion::schema::load(&token, database_id.trim())
        .await
        .map_err(|e| format!("Notion rejected the database: {e}"))?;
    let properties = crate::notion::detect::detect_properties(&schema);
    let status_map = crate::notion::detect::detect_status_map(&schema);

    // Excluding the completion state is what keeps finished work off Home. Detected
    // rather than assumed to be called "Done".
    let exclude_statuses = if status_map.done.is_empty() {
        vec![]
    } else {
        vec![status_map.done.clone()]
    };

    let template = template_page_id
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    // Validate it now: a template id that cannot be read fails at explorer→task
    // conversion, long after setup, with nothing pointing back here.
    if let Some(id) = &template {
        crate::notion::body::template_markdown(id, &token)
            .await
            .map_err(|e| format!("That template page could not be read: {e}"))?;
    }

    let cfg = Config {
        notion: NotionConfig {
            token: token.trim().to_string(),
            database_id: database_id.trim().to_string(),
            user_id: user_id.trim().to_string(),
            properties,
            status_map,
            filters: FilterConfig {
                exclude_statuses,
                filter_by_assignee: !user_id.trim().is_empty(),
            },
            task_template_page_id: template,
            default_project_id: None,
        },
        git: GitConfig { worktree_root: root },
        ui: UiConfig::default(),
    };

    crate::core::config::replace(cfg).map_err(|e| e.to_string())
}
