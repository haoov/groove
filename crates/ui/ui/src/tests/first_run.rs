//! The first run: Setup opens on its own while something is left, and says what.

use groove_controllers::AppState;
use groove_types::{Found, Tool};

use crate::Ui;
use crate::input::{Key, Modifiers};
use crate::tests::{app, press};
use crate::views::settings::Section;

fn tool(name: &'static str, required: bool, found: Option<(&str, Option<bool>)>) -> Tool {
    Tool {
        name,
        purpose: "a purpose",
        required,
        found: found.map(|(version, signed_in)| Found {
            version: version.into(),
            signed_in,
        }),
    }
}

/// The fixture once its check came back with these programs, its config as given.
fn checked(tools: Vec<Tool>, config: bool) -> AppState {
    let mut app = app();
    app.config.tools = Some(tools);
    app.config.checking = false;
    if config {
        app.config.config = Some(
            serde_json::from_str(r#"{ "git": { "worktree_root": "~/code" } }"#).expect("a config"),
        );
    } else {
        app.config.config = None;
    }
    app
}

fn ready() -> Vec<Tool> {
    vec![
        tool("git", true, Some(("2.47", None))),
        tool("claude", true, Some(("2.4", Some(true)))),
    ]
}

#[test]
fn setup_opens_once_when_the_first_check_finds_something_left() {
    let app = checked(
        vec![
            tool("git", true, None),
            tool("claude", true, Some(("2.4", Some(true)))),
        ],
        true,
    );
    let mut ui = Ui::default();
    ui.settled(&app);
    assert!(ui.settings.open, "a required program is missing");
    assert_eq!(ui.settings.section, Section::Setup);
    ui.settings.open = false;
    ui.settled(&app);
    assert!(!ui.settings.open, "only once a launch");
}

#[test]
fn a_ready_setup_or_a_user_who_acted_first_keeps_the_board() {
    let mut ui = Ui::default();
    ui.settled(&checked(ready(), true));
    assert!(!ui.settings.open, "nothing left");
    let signed_out = vec![tool("claude", true, Some(("2.4", Some(false))))];
    let app = checked(signed_out, true);
    let mut ui = Ui::default();
    press(Key::Down, Modifiers::default(), &mut ui, &app);
    ui.settled(&app);
    assert!(
        !ui.settings.open,
        "the user acted before the check came back"
    );
    let mut ui = Ui::default();
    let mut waiting = checked(ready(), false);
    waiting.config.checking = true;
    ui.settled(&waiting);
    assert!(!ui.settings.offered, "not decided while the check runs");
}

#[test]
fn the_nav_counts_what_setup_and_providers_still_need() {
    let app = checked(
        vec![tool("claude", true, Some(("2.4", Some(false))))],
        false,
    );
    assert_eq!(
        crate::views::settings::rows::setup_left(&app),
        2,
        "signed out, no config"
    );
    assert!(crate::views::settings::rows::no_source(&app));
    let mut ui = Ui::default();
    ui.settings.open = true;
    let texts = crate::tests::settings::drawn_whole(&app, &ui);
    let count = |label: &str| {
        let at = texts.iter().position(|one| one == label).expect(label);
        texts[at - 1].clone()
    };
    assert_eq!(count("Setup"), "2", "{texts:?}");
    assert_eq!(count("Providers"), "1", "{texts:?}");
}

#[test]
fn setup_reads_environment_then_login_then_files_then_next() {
    let app = checked(ready(), false);
    let ui = Ui::default();
    let rows = crate::views::settings::rows::rows(&app, &ui.settings);
    let mut groups: Vec<&str> = Vec::new();
    for one in rows.iter().filter(|one| one.section == Section::Setup) {
        if groups.last() != Some(&one.group) {
            groups.push(one.group);
        }
    }
    assert_eq!(groups, ["Environment", "Claude login", "Files", "Next"]);
}

#[test]
fn a_task_source_turns_on_and_off_by_its_toggle() {
    let app = checked(ready(), true);
    let mut ui = Ui::default();
    (ui.settings.open, ui.settings.section) = (true, Section::Providers);
    let (texts, hits) = crate::tests::settings::drawn(&app, &ui);
    let target = crate::hit::Target::SettingsTurnOn(groove_types::ProviderId::Notion);
    assert!(
        hits.rect_of(&target).is_some(),
        "the toggle turns Notion on"
    );
    assert!(
        !texts.iter().any(|one| one == "turn on" || one == "off"),
        "{texts:?}"
    );
}

#[test]
fn each_source_sums_up_its_gaps_and_splits_its_mapping_under_headings() {
    let app = crate::tests::bar::sourced(checked(ready(), true), true, true);
    let mut ui = Ui::default();
    (ui.settings.open, ui.settings.section) = (true, Section::Providers);
    let texts = crate::tests::settings::drawn_whole(&app, &ui);
    for heading in ["Notion", "Notion fields", "GitHub", "GitHub fields"] {
        assert!(
            texts.iter().any(|one| one == heading),
            "{heading}: {texts:?}"
        );
    }
    let gaps = crate::views::settings::rows::source_gaps(&app);
    assert!(gaps > 0, "the fixture leaves gaps");
    assert!(
        texts.iter().any(|one| one.ends_with(" gaps")),
        "the source row sums them up: {texts:?}"
    );
    let at = texts
        .iter()
        .position(|one| one == "Providers")
        .expect("the nav row");
    assert_eq!(texts[at - 1], gaps.to_string(), "the nav counts them");
}
