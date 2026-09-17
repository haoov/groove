use crate::{Config, ConfigView, ThemeName};

const LEGACY_FILE: &str = r#"{
  "notion": {
    "token": "secret_abc",
    "database_id": "db",
    "user_id": "u",
    "properties": { "status": "Status", "priority": "Priority", "sprint": null, "project": null, "assignee": "Owner" },
    "status_map": { "ready": "Ready", "in_progress": "In progress", "done": "Done" },
    "filters": { "exclude_statuses": ["Done"] }
  },
  "git": { "worktree_root": "/home/me/worktrees" },
  "ui": { "font_size": 15, "theme": "latte", "font_family": "", "agent_font_family": "", "suggest_actions": true }
}"#;

#[test]
fn a_legacy_file_loads_with_defaults_for_the_new_block() {
    let config: Config = serde_json::from_str(LEGACY_FILE).unwrap();
    assert_eq!(config.ui.theme, ThemeName::Latte);
    assert_eq!(config.ui.font_size, 15.0);
    assert_eq!(config.preferences.poll_interval_secs, 60);
    assert_eq!(config.preferences.thresholds.review_waiting_days, 3);
    assert!(!config.preferences.auto_approve_default);
    assert!(config.github.is_none());
    for (name, dark) in [
        ("latte", false),
        ("frappe", true),
        ("macchiato", true),
        ("mocha", true),
    ] {
        let theme: ThemeName = serde_json::from_str(&format!("\"{name}\"")).unwrap();
        assert_eq!(theme.is_dark(), dark, "{name}");
    }
}

#[test]
fn the_token_never_reaches_debug_or_the_view() {
    let config: Config = serde_json::from_str(LEGACY_FILE).unwrap();
    let debug = format!("{config:?}");
    assert!(!debug.contains("secret_abc"), "{debug}");
    assert!(debug.contains("<redacted>"));
    let view = serde_json::to_string(&ConfigView::from(config.clone())).unwrap();
    assert!(!view.contains("secret_abc"));
    let disk = serde_json::to_string(&config).unwrap();
    assert!(disk.contains("secret_abc"));
}
