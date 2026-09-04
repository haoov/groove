//! The skills a Groove agent can invoke, and the plugin dirs that carry them.
//!
//! Groove does not run skills — Claude Code does. A skill is a `SKILL.md` inside a
//! plugin directory, and `--plugin-dir` loads that plugin for ONE session. That
//! flag is the whole gate: a Groove skill reaches a Groove agent and nothing else,
//! so a skill may assume `mcp__groove__*` exists without checking for it.
//!
//! A skill says WHAT to do. Every rule about how to WRITE a commit message, MR
//! text, an annotation or a task body stays in `mcp_server/tools/definitions.rs`,
//! which the agent reads at the moment of the call. Do not restate them here.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::core::db::models::{Session, SessionKind};

/// Plugin names, and the agent-facing namespaces (`groove:start-task`). Renaming one
/// breaks every button.
pub const CORE_PLUGIN: &str = "groove";
pub const USER_PLUGIN: &str = "user";

/// Core skills, compiled in; the on-disk plugin is rewritten from these at startup.
const CORE_SKILLS: &[(&str, &str)] = &[
    ("close-task", include_str!("core/close-task.md")),
    ("co-review", include_str!("core/co-review.md")),
    ("create-task", include_str!("core/create-task.md")),
    ("fix-ci", include_str!("core/fix-ci.md")),
    ("fix-notes", include_str!("core/fix-notes.md")),
    ("new-skill", include_str!("core/new-skill.md")),
    ("save-task", include_str!("core/save-task.md")),
    ("start-task", include_str!("core/start-task.md")),
];

/// Appended to the agent's system prompt at every launch. See `core_prompt`.
const CORE_PROMPT: &str = include_str!("prompt.md");

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct AgentSkill {
    /// `groove:start-task` — what gets sent to the agent, and the UI's key.
    pub id: String,
    pub plugin: String,
    pub name: String,
    /// For the MODEL: what the skill does and when to invoke it. Also shown in
    /// Claude's slash menu.
    pub description: String,
    /// The one line the UI shows. Empty when unset — never the description.
    pub hint: String,
    pub label: String,
    /// Kinds whose UI offers it. Empty means every kind.
    pub kinds: Vec<SessionKind>,
    /// A user skill — the manager may edit or delete it.
    pub editable: bool,
}

// ─── The core prompt ──────────────────────────────────────────────────────────

/// The system prompt appended at every launch, with the session's identity
/// interpolated. Only STABLE identity — repos, worktrees and MRs drift.
pub fn core_prompt(task_id: &str, session: Option<&Session>) -> String {
    let who = match session {
        Some(s) => format!("session {} ({}): \"{}\"", s.id, kind_word(s.kind), s.title),
        None => format!("session {task_id}"),
    };
    CORE_PROMPT.replace("{{session}}", &who)
}

/// The word the prompt and the tools both use for a kind.
fn kind_word(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::Task => "task",
        SessionKind::Explorer => "explorer",
        SessionKind::Review => "review",
    }
}

// ─── Directories ──────────────────────────────────────────────────────────────

/// `<app data>/plugins/groove` — rewritten on every startup.
fn core_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    use tauri::Manager;
    Ok(app.path().app_data_dir()?.join("plugins").join(CORE_PLUGIN))
}

/// `<config>/user-skills`. The `skills/` inside is the plugin format's nesting.
pub fn user_dir() -> Option<PathBuf> {
    crate::core::config::dir().map(|d| d.join("user-skills"))
}

/// Put both plugin dirs in the state the launcher expects. Once, at startup.
pub fn sync(app: &tauri::AppHandle) -> anyhow::Result<()> {
    sync_core_at(&core_dir(app)?)?;
    if let Some(dir) = user_dir() {
        ensure_user_at(&dir)?;
    }
    Ok(())
}

/// Replace the core plugin from `CORE_SKILLS`, deleting skill dirs no longer in it.
fn sync_core_at(dir: &Path) -> anyhow::Result<()> {
    write_manifest(dir, CORE_PLUGIN, "Groove workbench actions")?;
    let skills = dir.join("skills");
    std::fs::create_dir_all(&skills)?;

    for name in skill_names(dir) {
        if !CORE_SKILLS.iter().any(|(n, _)| *n == name) {
            std::fs::remove_dir_all(skills.join(&name))?;
        }
    }
    for (name, body) in CORE_SKILLS {
        let skill = skills.join(name);
        std::fs::create_dir_all(&skill)?;
        std::fs::write(skill.join("SKILL.md"), body)?;
    }
    Ok(())
}

/// Create the user plugin. The manifest is rewritten every time; `skills/` never is.
fn ensure_user_at(dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir.join("skills"))?;
    write_manifest(dir, USER_PLUGIN, "Your own Groove actions")
}

fn write_manifest(dir: &Path, name: &str, description: &str) -> anyhow::Result<()> {
    let meta = dir.join(".claude-plugin");
    std::fs::create_dir_all(&meta)?;
    let body = serde_json::json!({
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
        "description": description,
        "author": { "name": "Groove" },
    });
    std::fs::write(meta.join("plugin.json"), serde_json::to_string_pretty(&body)?)?;
    Ok(())
}

/// The `--plugin-dir` arguments for one launch. A plugin with no skills is skipped.
pub fn plugin_dirs(app: &tauri::AppHandle) -> Vec<String> {
    [core_dir(app).ok(), user_dir()]
        .into_iter()
        .flatten()
        .filter(|dir| !skill_names(dir).is_empty())
        .map(|dir| dir.to_string_lossy().to_string())
        .collect()
}

// ─── Reading ──────────────────────────────────────────────────────────────────

/// Every skill both plugins offer, core first.
pub fn list(app: &tauri::AppHandle) -> Vec<AgentSkill> {
    let mut out = Vec::new();
    if let Ok(dir) = core_dir(app) {
        out.extend(read_plugin(&dir, CORE_PLUGIN, false));
    }
    if let Some(dir) = user_dir() {
        out.extend(read_plugin(&dir, USER_PLUGIN, true));
    }
    out
}

/// Skill directory names, sorted so the UI order does not depend on the filesystem.
fn skill_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join("skills"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

fn read_plugin(dir: &Path, plugin: &str, editable: bool) -> Vec<AgentSkill> {
    skill_names(dir)
        .into_iter()
        .filter_map(|name| {
            let body = std::fs::read_to_string(dir.join("skills").join(&name).join("SKILL.md")).ok()?;
            Some(parse(plugin, &name, &body, editable))
        })
        .collect()
}

/// Read a `SKILL.md`'s front matter. A hand parser: every key is a single-line
/// scalar. The DIRECTORY name is the identity, never the `name:` key.
fn parse(plugin: &str, name: &str, body: &str, editable: bool) -> AgentSkill {
    let front = front_matter(body);
    let kinds = parse_kinds(field(front, "groove-kinds").as_deref());
    AgentSkill {
        id: format!("{plugin}:{name}"),
        plugin: plugin.to_string(),
        name: name.to_string(),
        description: field(front, "description").unwrap_or_default(),
        hint: field(front, "groove-hint").unwrap_or_default(),
        label: field(front, "groove-label").unwrap_or_else(|| name.replace('-', " ")),
        kinds,
        editable,
    }
}

/// The text between the opening `---` and the next one. Empty when there is none.
fn front_matter(body: &str) -> &str {
    let rest = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))
        .unwrap_or("");
    match rest.find("\n---") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// One `key: value` line. Split on the FIRST colon — a description contains them.
fn field(front: &str, key: &str) -> Option<String> {
    front.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if k.trim() != key {
            return None;
        }
        let v = v.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Unknown kinds are dropped, not fatal.
fn parse_kinds(raw: Option<&str>) -> Vec<SessionKind> {
    raw.into_iter()
        .flat_map(|v| v.split(','))
        .filter_map(|s| match s.trim().to_ascii_lowercase().as_str() {
            "task" => Some(SessionKind::Task),
            "explorer" => Some(SessionKind::Explorer),
            "review" => Some(SessionKind::Review),
            _ => None,
        })
        .collect()
}

// ─── The user's own skills ────────────────────────────────────────────────────

/// Lowercase, digits and dashes only — the name is joined onto the plugin dir as a path.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The `SKILL.md` path for a `plugin:name` id.
fn skill_path(app: &tauri::AppHandle, id: &str) -> anyhow::Result<PathBuf> {
    let (plugin, name) = id
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("not a skill id: {id}"))?;
    if !valid_name(name) {
        anyhow::bail!("not a skill name: {name}");
    }
    let dir = match plugin {
        CORE_PLUGIN => core_dir(app)?,
        USER_PLUGIN => user_dir().ok_or_else(|| anyhow::anyhow!("no config directory"))?,
        other => anyhow::bail!("unknown plugin: {other}"),
    };
    Ok(dir.join("skills").join(name).join("SKILL.md"))
}

/// The user plugin's skills dir.
fn user_skills_dir() -> anyhow::Result<PathBuf> {
    Ok(user_dir()
        .ok_or_else(|| anyhow::anyhow!("no config directory"))?
        .join("skills"))
}

/// Write one user skill. With `previous`, remove the dir it was renamed from — AFTER
/// the new one is on disk.
fn save_user_skill_at(root: &Path, name: &str, body: &str, previous: Option<&str>) -> anyhow::Result<()> {
    if !valid_name(name) {
        anyhow::bail!("a name is lowercase letters, digits and dashes: `{name}`");
    }
    let dir = root.join(name);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("SKILL.md"), body)?;

    if let Some(previous) = previous.filter(|p| *p != name && valid_name(p)) {
        std::fs::remove_dir_all(root.join(previous))?;
    }
    Ok(())
}

/// `claude plugin validate` on the user plugin, as text. None when `claude` is not on PATH.
async fn validate_user_plugin() -> Option<String> {
    let dir = user_dir()?;
    let out = tokio::process::Command::new(crate::agent_manager::resolve_claude_bin())
        .args(["plugin", "validate", &dir.to_string_lossy()])
        .output()
        .await
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

// ─── What the agent writes ────────────────────────────────────────────────────

/// One user skill's raw `SKILL.md`. User skills only — core ones are rewritten at startup.
pub fn read_user_skill(name: &str) -> anyhow::Result<String> {
    if !valid_name(name) {
        anyhow::bail!("not a skill name: `{name}`");
    }
    let path = user_skills_dir()?.join(name).join("SKILL.md");
    std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("no user skill `{name}`: {e}"))
}

/// The approved `skill.save` op. Refuses to overwrite an existing name unless
/// `previous` names it. Returns what `claude plugin validate` said.
pub(crate) async fn save_user_skill_impl(
    payload: serde_json::Value,
) -> anyhow::Result<Option<String>> {
    let name = payload["name"].as_str().unwrap_or_default();
    let body = payload["body"].as_str().unwrap_or_default();
    let previous = payload["previous"].as_str().filter(|p| !p.is_empty());
    if body.trim().is_empty() {
        anyhow::bail!("a skill with no body is not a skill");
    }

    let root = user_skills_dir()?;
    if root.join(name).join("SKILL.md").exists() && previous != Some(name) {
        anyhow::bail!(
            "a user skill named `{name}` already exists — pass previous: \"{name}\" to replace it, or choose another name"
        );
    }
    save_user_skill_at(&root, name, body, previous)?;
    Ok(validate_user_plugin().await)
}

// ─── IPC ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_agent_skills(app: tauri::AppHandle) -> Result<Vec<AgentSkill>, String> {
    Ok(list(&app))
}

/// The raw `SKILL.md` behind a skill id, core or user.
#[tauri::command]
pub async fn read_agent_skill(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let path = skill_path(&app, &id).map_err(|e| e.to_string())?;
    std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))
}

/// Create or replace a user skill from the Settings editor. A local write — not bridged.
#[tauri::command]
pub async fn save_user_skill(
    name: String,
    body: String,
    previous_name: Option<String>,
) -> Result<Option<String>, String> {
    let root = user_skills_dir().map_err(|e| e.to_string())?;
    save_user_skill_at(&root, &name, &body, previous_name.as_deref()).map_err(|e| e.to_string())?;
    Ok(validate_user_plugin().await)
}

#[tauri::command]
pub async fn delete_user_skill(name: String) -> Result<(), String> {
    if !valid_name(&name) {
        return Err(format!("not a skill name: {name}"));
    }
    let dir = user_skills_dir().map_err(|e| e.to_string())?.join(&name);
    std::fs::remove_dir_all(&dir).map_err(|e| format!("Cannot delete {}: {e}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("groove-skills-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn write_skill(dir: &Path, name: &str, body: &str) {
        let skill = dir.join("skills").join(name);
        std::fs::create_dir_all(&skill).unwrap();
        std::fs::write(skill.join("SKILL.md"), body).unwrap();
    }

    fn session(kind: SessionKind) -> Session {
        Session {
            id: "PLAT-42".to_string(),
            kind,
            title: "Rework the agent pill".to_string(),
            external_id: None,
            review_project: None,
            review_iid: None,
            created_at: 0,
        }
    }

    #[test]
    fn the_prompt_names_the_session_and_its_kind() {
        let p = core_prompt("PLAT-42", Some(&session(SessionKind::Review)));
        assert!(p.starts_with(
            "You are the Groove agent for session PLAT-42 (review): \"Rework the agent pill\"."
        ));
        assert!(!p.contains("{{"), "a placeholder survived interpolation");
    }

    /// A missing session row still yields a prompt that names the task.
    #[test]
    fn the_prompt_falls_back_to_the_bare_id() {
        let p = core_prompt("PLAT-42", None);
        assert!(p.starts_with("You are the Groove agent for session PLAT-42."));
        assert!(!p.contains("{{"));
    }

    /// A misspelt kind would silently offer the skill everywhere.
    #[test]
    fn every_core_skill_carries_a_description_and_real_kinds() {
        for (name, body) in CORE_SKILLS {
            let s = parse(CORE_PLUGIN, name, body, false);
            assert!(!s.description.is_empty(), "{name} has no description");
            // The description must say WHEN to invoke, not only what.
            assert!(
                s.description.contains("Use when"),
                "{name}'s description names no trigger"
            );
            assert!(!s.hint.is_empty(), "{name} has no groove-hint");
            assert!(s.hint.len() <= 60, "{name}'s hint is too long for a tooltip");
            assert!(!s.kinds.is_empty(), "{name} declares no groove-kinds");
            let declared = field(front_matter(body), "groove-kinds").unwrap();
            assert_eq!(
                s.kinds.len(),
                declared.split(',').count(),
                "{name} has an unreadable kind in `{declared}`"
            );
        }
    }

    /// A skill body is prose: a renamed tool leaves it pointing at nothing.
    #[test]
    fn every_tool_a_core_skill_names_exists() {
        // Backticked snake_case that is a payload field, not a tool.
        const FIELDS: &[&str] = &["unlogged_hours"];
        let tools: Vec<String> = crate::mcp_server::mcp_tool_definitions()
            .iter()
            .filter_map(|t| t["name"].as_str().map(str::to_string))
            .collect();

        for (name, body) in CORE_SKILLS {
            for token in body.split('`').skip(1).step_by(2) {
                let looks_like_tool = token.contains('_')
                    && token.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
                if !looks_like_tool || FIELDS.contains(&token) {
                    continue;
                }
                assert!(
                    tools.iter().any(|t| t == token),
                    "{name} names `{token}`, which is not a tool"
                );
            }
        }
    }

    #[test]
    fn parses_front_matter_and_groove_keys() {
        let s = parse(
            "groove",
            "sample-skill",
            "---\nname: sample-skill\ndescription: Do a thing: and another.\ngroove-kinds: task, review\ngroove-label: do a thing\ngroove-hint: Does a thing.\n---\n\nBody.\n",
            false,
        );
        assert_eq!(s.id, "groove:sample-skill");
        // Split on the FIRST colon, so a description keeps its own.
        assert_eq!(s.description, "Do a thing: and another.");
        assert_eq!(s.hint, "Does a thing.");
        assert_eq!(s.label, "do a thing");
        assert_eq!(s.kinds, vec![SessionKind::Task, SessionKind::Review]);
        assert!(!s.editable);
    }

    #[test]
    fn defaults_when_groove_keys_are_absent() {
        let s = parse("user", "deploy-check", "---\ndescription: Check it.\n---\n", true);
        assert_eq!(s.id, "user:deploy-check");
        assert_eq!(s.label, "deploy check");
        // No hint means no line, never the description.
        assert_eq!(s.hint, "");
        // No kinds means every kind offers it, not none.
        assert!(s.kinds.is_empty());
        assert!(s.editable);
    }

    #[test]
    fn survives_a_skill_with_no_front_matter() {
        let s = parse("user", "raw", "Just a body.\n", true);
        assert_eq!(s.description, "");
        assert_eq!(s.label, "raw");
    }

    #[test]
    fn a_typo_in_kinds_drops_only_that_kind() {
        assert_eq!(parse_kinds(Some("task, tsak, review")), vec![SessionKind::Task, SessionKind::Review]);
    }

    #[test]
    fn sync_core_writes_the_manifest_and_removes_a_stale_skill() {
        let dir = tmp("core");
        write_skill(&dir, "dropped-last-release", "---\ndescription: Gone.\n---\n");
        sync_core_at(&dir).unwrap();

        let manifest = std::fs::read_to_string(dir.join(".claude-plugin/plugin.json")).unwrap();
        assert!(manifest.contains("\"name\": \"groove\""));
        assert!(!dir.join("skills/dropped-last-release").exists());
        for (name, _) in CORE_SKILLS {
            assert!(dir.join("skills").join(name).join("SKILL.md").is_file());
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_user_rewrites_the_manifest_and_keeps_the_skills() {
        let dir = tmp("user");
        write_skill(&dir, "mine", "---\ndescription: Mine.\n---\n");
        ensure_user_at(&dir).unwrap();
        ensure_user_at(&dir).unwrap();

        let listed = read_plugin(&dir, USER_PLUGIN, true);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "user:mine");
        assert!(dir.join(".claude-plugin/plugin.json").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A rejected name is a rejected path escape.
    #[test]
    fn a_name_is_a_safe_directory_name() {
        for ok in ["deploy-check", "a", "x2", "check-prod-2"] {
            assert!(valid_name(ok), "{ok} should be valid");
        }
        for bad in ["", "..", "../escape", "a/b", "Caps", "-lead", "sp ace", "dot.name"] {
            assert!(!valid_name(bad), "{bad} should be rejected");
        }
        assert!(!valid_name(&"a".repeat(65)));
    }

    #[test]
    fn saving_a_renamed_skill_moves_it() {
        let root = tmp("rename").join("skills");
        save_user_skill_at(&root, "old", "---\ndescription: One.\n---\n", None).unwrap();
        save_user_skill_at(&root, "new", "---\ndescription: Two.\n---\n", Some("old")).unwrap();
        assert!(!root.join("old").exists());
        assert_eq!(
            std::fs::read_to_string(root.join("new/SKILL.md")).unwrap(),
            "---\ndescription: Two.\n---\n"
        );
        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn editing_a_skill_replaces_its_body() {
        let root = tmp("resave").join("skills");
        save_user_skill_at(&root, "same", "one", None).unwrap();
        save_user_skill_at(&root, "same", "two", Some("same")).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("same/SKILL.md")).unwrap(), "two");
        std::fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_bad_name_writes_nothing() {
        let root = tmp("badname").join("skills");
        assert!(save_user_skill_at(&root, "../escape", "x", None).is_err());
        assert!(!root.exists());
    }
}
