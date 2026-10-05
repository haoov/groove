use crate::{Config, ConfigView, FontFamily, ThemeName};

const FILE: &str = r#"{
  "notion": {
    "token": "secret_abc",
    "database_id": "db",
    "user_id": "u",
    "properties": { "status": "Status", "priority": "Priority", "due": "Due date" },
    "status_map": { "ready": ["Ready"], "in_progress": ["In progress"], "done": ["Done", "Archived"] },
    "priority_map": { "high": ["P1"], "medium": ["P2"], "low": ["P3"] },
    "filters": { "exclude_statuses": ["Done"] }
  },
  "git": { "worktree_root": "/home/me/worktrees" },
  "ui": { "font_size": 15, "theme": "latte", "font_family": "", "agent_font_family": "" }
}"#;

#[test]
fn a_file_loads_with_defaults_for_everything_it_leaves_out() {
    let config: Config = serde_json::from_str(FILE).unwrap();
    assert_eq!(config.ui.theme, ThemeName::Latte);
    assert_eq!(config.ui.font_size, 15.0);
    assert_eq!(config.preferences.poll_interval_secs, 60);
    assert_eq!(config.preferences.thresholds.review_waiting_days, 3);
    assert!(!config.preferences.auto_approve_default);
    assert!(config.github.is_none());
    let notion = config.notion.as_ref().expect("the source");
    assert_eq!(notion.properties.due.as_deref(), Some("Due date"));
    assert_eq!(notion.properties.estimate, None, "a gap, not an error");
    assert_eq!(
        notion.status_map.intent("Archived"),
        Some(crate::StatusIntent::Done),
        "a status the map names alongside another"
    );
    assert_eq!(
        notion.status_map.label(crate::StatusIntent::Done),
        Some("Done")
    );
    assert_eq!(notion.priority_map.level("P1"), Some(crate::Priority::High));
    assert_eq!(notion.priority_map.level("P9"), None);
    for name in ["latte", "frappe", "macchiato", "mocha"] {
        let read = serde_json::from_str::<ThemeName>(&format!("\"{name}\""));
        assert!(read.is_ok(), "{name} reads as a theme");
    }
}

#[test]
fn the_token_never_reaches_debug_or_the_view() {
    let config: Config = serde_json::from_str(FILE).unwrap();
    let debug = format!("{config:?}");
    assert!(!debug.contains("secret_abc"), "{debug}");
    assert!(debug.contains("<redacted>"));
    let view = serde_json::to_string(&ConfigView::from(config.clone())).unwrap();
    assert!(!view.contains("secret_abc"));
    let disk = serde_json::to_string(&config).unwrap();
    assert!(disk.contains("secret_abc"));
}

#[test]
fn a_family_reads_back_from_what_it_stores_and_any_other_name_reads_as_plex() {
    for one in FontFamily::MONO {
        assert_eq!(FontFamily::named(one.stored(), &FontFamily::MONO), one);
    }
    assert_eq!(
        FontFamily::named("Fira Code", &FontFamily::MONO),
        FontFamily::Plex
    );
    assert_eq!(
        FontFamily::named("Lilex", &FontFamily::UI),
        FontFamily::Plex
    );
}

#[test]
fn a_file_naming_the_agent_font_reads_it_as_the_mono_font() {
    let file = r#"{ "font_family": "", "agent_font_family": "Lilex" }"#;
    let ui: crate::UiConfig = serde_json::from_str(file).unwrap();
    assert_eq!(ui.mono_font_family, "Lilex");
}
