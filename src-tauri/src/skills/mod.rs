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

/// Plugin names, which are also the agent-facing namespaces — `groove:open-mr`,
/// `user:deploy-check`. Renaming one breaks every button built on the old prefix.
pub const CORE_PLUGIN: &str = "groove";
pub const USER_PLUGIN: &str = "user";

/// Core skills, compiled in. Shipping them as source rather than seeding a
/// directory means an app update replaces them outright — nothing to migrate, and
/// no half-edited core skill can survive an upgrade.
const CORE_SKILLS: &[(&str, &str)] = &[
    ("co-review", include_str!("core/co-review.md")),
    ("create-task", include_str!("core/create-task.md")),
    ("fix-notes", include_str!("core/fix-notes.md")),
    ("new-skill", include_str!("core/new-skill.md")),
    ("open-mr", include_str!("core/open-mr.md")),
    ("start-task", include_str!("core/start-task.md")),
    ("update-task", include_str!("core/update-task.md")),
];

/// Appended to the agent's system prompt at every launch. See `core_prompt`.
const CORE_PROMPT: &str = include_str!("prompt.md");

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct AgentSkill {
    /// `groove:open-mr` — what gets sent to the agent, and the UI's key.
    pub id: String,
    pub plugin: String,
    pub name: String,
    /// Written for the MODEL: what the skill does and when to reach for it, so
    /// Claude Code can invoke it off what the user typed. Its slash menu renders
    /// it too, so it stays readable — but it is too long for a tooltip.
    pub description: String,
    /// The one line the UI shows. `groove-hint`, falling back to the description
    /// for a user skill that sets none.
    pub hint: String,
    pub label: String,
    /// Kinds whose UI offers it. Empty means every kind.
    pub kinds: Vec<SessionKind>,
    /// `groove-hidden` — kept out of the action menu. Still a skill: the agent
    /// invokes it, another skill hands off to it, the user can type it. For the
    /// steps that belong inside a bigger one rather than beside it.
    pub hidden: bool,
    /// A user skill — the manager may edit or delete it.
    pub editable: bool,
}

// ─── The core prompt ──────────────────────────────────────────────────────────

/// What every Groove agent is told about the app it runs inside.
///
/// Passed per launch with `--append-system-prompt-file`, never baked into the
/// conversation, so a resumed session picks up the current text and an app update
/// reaches every session already open.
///
/// Only STABLE identity is interpolated. Repos, worktrees and MRs drift inside a
/// long session, and a stale system prompt lies louder than a missing one — those
/// stay in `get_active_task`, which is read at the moment it is needed.
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

/// `<config>/user-skills`. The `skills/` inside it is the plugin format's nesting,
/// not ours; the manager shows only what lives under it.
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

/// Replace the core plugin from `CORE_SKILLS`.
///
/// Stale skill dirs are deleted, not left: a skill renamed or dropped in a release
/// would otherwise keep loading forever, and the agent would keep being offered it.
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

/// Create the user plugin without touching its skills.
///
/// The manifest is rewritten every time and the skills never are: the wrapper is
/// Groove's, `skills/` is the user's. A missing or edited manifest stops the whole
/// plugin from loading, which reads as "my actions vanished", so it is not left to
/// chance.
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

/// The `--plugin-dir` arguments for one launch.
///
/// A plugin with no skill is skipped — loading an empty one costs a startup
/// warning for nothing, and the user plugin is empty until they write their first.
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

/// Read one `SKILL.md`'s front matter.
///
/// A hand parser, not YAML: every key Groove reads is a single-line scalar, with a
/// comma list where it needs several. Claude Code passes the `groove-*` keys
/// through untouched — `claude plugin validate` accepts them.
///
/// The DIRECTORY name is the identity, never the `name:` key: it is what
/// `/groove:<name>` resolves against, so a stale key cannot rename a skill.
fn parse(plugin: &str, name: &str, body: &str, editable: bool) -> AgentSkill {
    let front = front_matter(body);
    let kinds = parse_kinds(field(front, "groove-kinds").as_deref());
    AgentSkill {
        id: format!("{plugin}:{name}"),
        plugin: plugin.to_string(),
        name: name.to_string(),
        description: field(front, "description").unwrap_or_default(),
        hint: field(front, "groove-hint")
            .or_else(|| field(front, "description"))
            .unwrap_or_default(),
        label: field(front, "groove-label").unwrap_or_else(|| name.replace('-', " ")),
        kinds,
        hidden: field(front, "groove-hidden").as_deref() == Some("true"),
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

/// Unknown names are dropped rather than failing the skill: a typo should cost the
/// button, not the whole entry.
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

/// A skill name as it lands on disk and in `/user:<name>`.
///
/// Lowercase, digits and dashes only. It is a directory name joined onto the
/// plugin dir, so anything else is either a path escape or a slash command the
/// user cannot type.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The directory of one skill, by `plugin:name`. Errors rather than guessing: an
/// unknown plugin or a name that is not a plain directory has no path.
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

/// The user plugin's skills dir, or the error that says the config never loaded.
fn user_skills_dir() -> anyhow::Result<PathBuf> {
    Ok(user_dir()
        .ok_or_else(|| anyhow::anyhow!("no config directory"))?
        .join("skills"))
}

/// Write one user skill, optionally replacing the one it was renamed from.
///
/// The old directory goes only after the new one is on disk, so a failed write
/// leaves the original skill intact rather than losing both.
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

/// `claude plugin validate` on the user plugin, as text for the UI.
///
/// It reads the SKILL.md too, not only the manifest: a missing front-matter block
/// or description comes back as a warning. Best effort — no `claude` on PATH costs
/// the check, never the save.
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

/// One user skill's raw `SKILL.md`, for an agent about to edit it.
///
/// The user's own only. The core plugin is rewritten from `CORE_SKILLS` at every
/// startup, so handing the agent a core skill to edit would promise an edit the
/// next launch silently erases.
pub fn read_user_skill(name: &str) -> anyhow::Result<String> {
    if !valid_name(name) {
        anyhow::bail!("not a skill name: `{name}`");
    }
    let path = user_skills_dir()?.join(name).join("SKILL.md");
    std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("no user skill `{name}`: {e}"))
}

/// The approved `skill.save` op. Returns what `claude plugin validate` said.
///
/// Writing over an existing name is refused unless the call says it is REPLACING
/// that skill: an agent picking a name that happens to be taken would otherwise
/// overwrite something the user wrote, with the same confirmation text either way.
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

/// The raw `SKILL.md` behind a skill id. Core skills read too — the manager offers
/// one as the starting point for a user's own.
#[tauri::command]
pub async fn read_agent_skill(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let path = skill_path(&app, &id).map_err(|e| e.to_string())?;
    std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))
}

/// Create or replace a user skill. Returns what `claude plugin validate` said, or
/// null when it could not run.
///
/// A local file, so it does NOT go through the approvals bridge: that gate is for
/// writes leaving the machine.
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

    /// A session row can be missing (a race at first launch); the prompt still has
    /// to name something, because every tool default is "your own task".
    #[test]
    fn the_prompt_falls_back_to_the_bare_id() {
        let p = core_prompt("PLAT-42", None);
        assert!(p.starts_with("You are the Groove agent for session PLAT-42."));
        assert!(!p.contains("{{"));
    }

    /// A typo in `groove-kinds` costs a button silently — the skill just shows up
    /// in every session instead of one. Cheap to guard, invisible to debug.
    #[test]
    fn every_core_skill_carries_a_description_and_real_kinds() {
        for (name, body) in CORE_SKILLS {
            let s = parse(CORE_PLUGIN, name, body, false);
            assert!(!s.description.is_empty(), "{name} has no description");
            // The description is what Claude Code matches the user's words against,
            // so it has to say WHEN to reach for the skill and not only what it does.
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

    #[test]
    fn parses_front_matter_and_groove_keys() {
        let s = parse(
            "groove",
            "open-mr",
            "---\nname: open-mr\ndescription: Open an MR: for this branch.\ngroove-kinds: task, review\ngroove-label: open MR\ngroove-hint: Open an MR.\n---\n\nBody.\n",
            false,
        );
        assert_eq!(s.id, "groove:open-mr");
        // Split on the FIRST colon, so a description keeps its own.
        assert_eq!(s.description, "Open an MR: for this branch.");
        assert_eq!(s.hint, "Open an MR.");
        assert_eq!(s.label, "open MR");
        assert!(!s.hidden);
        assert_eq!(s.kinds, vec![SessionKind::Task, SessionKind::Review]);
        assert!(!s.editable);
    }

    /// A user skill that sets no `groove-hint` still needs a tooltip, and its
    /// description is the only line it has.
    #[test]
    fn the_hint_falls_back_to_the_description() {
        let s = parse("user", "deploy-check", "---\ndescription: Check it.\n---\n", true);
        assert_eq!(s.hint, "Check it.");
    }

    #[test]
    fn defaults_when_groove_keys_are_absent() {
        let s = parse("user", "deploy-check", "---\ndescription: Check it.\n---\n", true);
        assert_eq!(s.id, "user:deploy-check");
        assert_eq!(s.label, "deploy check");
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

    /// Hiding a skill takes it out of the menu and nothing else — it stays
    /// loaded, invocable and listed in the manager.
    #[test]
    fn a_skill_can_be_hidden_from_the_menu() {
        let s = parse("groove", "handoff-step", "---\ndescription: D.\ngroove-hidden: true\n---\n", false);
        assert!(s.hidden);
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
    fn sync_core_is_idempotent() {
        let dir = tmp("idem");
        sync_core_at(&dir).unwrap();
        sync_core_at(&dir).unwrap();
        assert_eq!(skill_names(&dir).len(), CORE_SKILLS.len());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ensure_user_rewrites_the_manifest_and_keeps_the_skills() {
        let dir = tmp("user");
        write_skill(&dir, "mine", "---\ndescription: Mine.\n---\n");
        std::fs::write(dir.join(".claude-plugin.tmp"), "").ok();
        ensure_user_at(&dir).unwrap();
        ensure_user_at(&dir).unwrap();

        let listed = read_plugin(&dir, USER_PLUGIN, true);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "user:mine");
        assert!(dir.join(".claude-plugin/plugin.json").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The name is joined onto the plugin dir, so a rejected name is a rejected
    /// path escape — not a style rule.
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

    /// Saving over itself must not delete what it just wrote.
    #[test]
    fn saving_under_the_same_name_keeps_the_skill() {
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

    #[test]
    fn a_plugin_with_no_skills_reads_as_empty() {
        let dir = tmp("empty");
        ensure_user_at(&dir).unwrap();
        assert!(skill_names(&dir).is_empty());
        assert!(read_plugin(&dir, USER_PLUGIN, true).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
