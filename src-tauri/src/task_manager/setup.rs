//! First run: what the machine is missing, and writing the config once.
//!
//! Everything the app needs beyond itself is checked here and reported in one
//! payload, so a new user sees the whole list at once instead of discovering each
//! missing tool through a failed action.

use serde::Serialize;

use crate::core::config::{Config, GitConfig, UiConfig};
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
    /// For tools that hold their own credentials: whether they are logged in.
    /// `None` when the tool is absent or has nothing to authenticate.
    ///
    /// Installed-but-not-logged-in is the state worth naming: every MR feature
    /// fails, and the CLI's own error ("not logged in") only appears once you try.
    pub authed: Option<bool>,
    /// The token's scopes, when the CLI reports them.
    ///
    /// `None` means UNKNOWN, not missing: a GH_TOKEN or a fine-grained PAT prints
    /// no scopes line, and treating that as missing would warn those users for
    /// ever. Only a `Some` that lacks a scope is worth acting on.
    pub scopes: Option<Vec<String>>,
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
/// extended, so a tool installed by Homebrew or npm is found even though a desktop
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
        std::process::Command::new(&name).args(["auth", "status"]).env("NO_COLOR", "1").output()
    })
    .await;

    let Ok(Ok(out)) = out else { return (Some(false), None) };
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

/// One clipboard tool is enough, so they are reported as a group: the app uses the
/// first tool that fits the session (wl-clipboard on Wayland, xclip/xsel on X11).
#[cfg(not(target_os = "macos"))]
fn clipboard_tools() -> Vec<ToolCheck> {
    vec![
        check("wl-copy", "Copy from the terminal on Wayland (wl-clipboard)", false),
        check("xclip", "Copy from the terminal on X11", false),
        check("xsel", "Copy from the terminal on X11 (alternative to xclip)", false),
    ]
}

/// `pbcopy`/`pbpaste` ship with macOS, so there is nothing to install or report.
#[cfg(target_os = "macos")]
fn clipboard_tools() -> Vec<ToolCheck> {
    vec![]
}

/// macOS uses `osascript`, which ships with the OS, so it reports nothing.
#[cfg(not(target_os = "macos"))]
fn notification_tools() -> Vec<ToolCheck> {
    vec![check(
        "notify-send",
        "Desktop notifications when the window is unfocused",
        false,
    )]
}

#[cfg(target_os = "macos")]
fn notification_tools() -> Vec<ToolCheck> {
    vec![]
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
        // Claude Code reports what the agent is doing by POSTing to the app from a
        // hook, and the hook command is a curl. Without it the agent still works,
        // but the dock and the console never say what it is doing.
        check("curl", "Agent status (waiting / working / idle) in the dock", false),
        // One CLI per forge, each owning its own auth. Only the forges you actually
        // have repos on matter, so both are optional.
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
        // `sh` is on every PATH; this is really checking `which` itself works.
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

    /// A token that prints no scopes line is UNKNOWN, not unscoped — warning
    /// those users about a missing scope would be permanent and wrong.
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

/// Write the initial config.
///
/// Property names and status values are DETECTED from the database rather than
/// asked for or defaulted: Notion already knows which property is the status and
/// which of its options mean to-do / in-progress / complete. They are still written
/// to the file, so a wrong detection can be corrected without a rebuild.
#[tauri::command]
pub async fn write_initial_config(setup: SetupRequest) -> Result<(), String> {
    let root = crate::core::fs::expand_tilde(setup.worktree_root.trim());
    if root.is_empty() {
        return Err("A worktree root is required — the directory repos are cloned into.".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| format!("Cannot create {root}: {e}"))?;

    let notion = match &setup.notion {
        Some(n) => Some(crate::provider::notion::setup::build_config(n).await?),
        None => None,
    };
    let github =
        setup.github.as_ref().map(|g| crate::provider::github::setup::build_config(g, None));

    let cfg = Config {
        notion,
        github,
        git: GitConfig { worktree_root: root },
        ui: UiConfig::default(),
    };
    if !crate::provider::has_task_source(&cfg) {
        return Err("Set up at least one task source.".into());
    }
    crate::core::config::replace(cfg).map_err(|e| e.to_string())
}

/// What the setup screen sends: a worktree root, plus whichever sources were
/// filled in.
#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct SetupRequest {
    pub worktree_root: String,
    pub notion: Option<NotionSetup>,
    pub github: Option<GithubSetup>,
}

/// Turn one task source on or off after first run.
///
/// Existing installs never see the setup screen again, so this is the only route
/// to adding a source to a machine that is already configured. `options` is the
/// provider's own setup payload (NotionSetup / GithubSetup as JSON); null works
/// for a source with nothing to fill in, and always for disabling.
///
/// The match is exhaustive over ProviderId ON PURPOSE: config fields are typed
/// per provider, so this is a sanctioned, compiler-enforced edit site.
#[tauri::command]
pub async fn set_task_source(
    provider: crate::provider::types::ProviderId,
    enabled: bool,
    options: serde_json::Value,
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<(), String> {
    use crate::provider::types::ProviderId;

    let mut cfg = crate::core::config::require().map_err(|e| e.to_string())?;
    match (provider, enabled) {
        (ProviderId::Notion, true) => {
            let setup: crate::provider::notion::setup::NotionSetup =
                serde_json::from_value(options).map_err(|e| format!("bad Notion setup: {e}"))?;
            cfg.notion = Some(crate::provider::notion::setup::build_config(&setup).await?);
        }
        (ProviderId::Notion, false) => cfg.notion = None,
        (ProviderId::Github, true) => {
            // Reconnecting keeps whatever was corrected by hand in the config
            // file, which is the only place those names can be corrected.
            let setup: crate::provider::github::setup::GithubSetup =
                serde_json::from_value(options).unwrap_or(crate::provider::github::setup::GithubSetup { host: None });
            cfg.github = Some(crate::provider::github::setup::build_config(&setup, cfg.github.take()));
        }
        (ProviderId::Github, false) => cfg.github = None,
    }
    if !crate::provider::has_task_source(&cfg) {
        return Err("That would leave no task source at all.".into());
    }
    crate::core::config::replace(cfg).map_err(|e| e.to_string())?;

    // A disabled source's mirror rows would keep rendering on Home forever —
    // its sync loop, the usual pruner, no longer runs. Checked-out tasks stay.
    if !enabled {
        crate::core::db::store::provider_tasks::prune_provider(&*pool, provider.as_str())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
