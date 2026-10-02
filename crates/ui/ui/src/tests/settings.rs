//! Settings: opened from the rail and the palette, over the whole window, its rows searched and set.

use groove_controllers::config_service::{Font, Preference};
use groove_controllers::{AppState, Command, config};
use groove_gfx::Fonts;
use groove_types::{Found, ProviderId, Secret, ThemeName, Tool};

use super::*;
use crate::hit::Target;
use crate::view;

fn drawn(app: &AppState, ui: &Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let texts = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    (texts, hits)
}

fn opened() -> (AppState, Ui) {
    let app = full_app();
    let mut ui = Ui::default();
    ui.settings.open = true;
    (app, ui)
}

#[test]
fn the_rail_s_footer_opens_settings_over_the_whole_window_and_esc_comes_back() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let footer = hits.rect_of(&Target::SettingsOpen).expect("the footer row");
    click(footer, &mut ui, &app, &hits);
    assert!(ui.settings.open);
    let (texts, hits) = drawn(&app, &ui);
    assert!(
        hits.rect_of(&Target::Board).is_none(),
        "the rail is not drawn"
    );
    assert!(texts.iter().any(|one| one == "Preferences"), "{texts:?}");
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(!ui.settings.open);
}

#[test]
fn a_search_finds_rows_of_every_section_and_names_the_group_of_each() {
    let (app, mut ui) = opened();
    let app = crate::tests::bar::sourced(app, true, true);
    ui.settings.search.set("token");
    let (texts, _) = drawn(&app, &ui);
    for shown in ["Notion", "GitHub", "Forge tokens", "gh", "glab"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    ui.settings.search.set("github status");
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "1 found"), "{texts:?}");
}

#[test]
fn a_step_up_asks_for_the_preference_one_step_more() {
    let (app, mut ui) = opened();
    let (_, hits) = drawn(&app, &ui);
    let at = app.config.preferences().poll_interval_secs;
    let more = Target::SetPreference(Preference::PollIntervalSecs(at + 10));
    let plus = hits.rect_of(&more).expect("the poll interval's +");
    let asked = click(plus, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(Preference::PollIntervalSecs(at + 10));
    assert_eq!(asked, [Command::Config(set)]);
}

#[test]
fn appearance_offers_every_theme_and_a_click_picks_one() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Appearance;
    let (texts, hits) = drawn(&app, &ui);
    for one in ThemeName::ALL {
        assert!(texts.iter().any(|t| t == one.label()), "{one:?}: {texts:?}");
    }
    let mocha = Target::SetPreference(Preference::Theme(ThemeName::Mocha));
    let at = hits.rect_of(&mocha).expect("mocha's word");
    let asked = click(at, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(Preference::Theme(ThemeName::Mocha));
    assert_eq!(asked, [Command::Config(set)]);
}

#[test]
fn each_font_has_its_own_size_and_a_step_up_asks_for_one_point_more() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Appearance;
    let (texts, hits) = drawn(&app, &ui);
    for row in ["ui size", "editor size", "terminal size"] {
        assert!(texts.iter().any(|t| t == row), "{row}: {texts:?}");
    }
    let at = app.config.terminal_size();
    let more = Preference::FontSize(Font::Terminal, at + 1.0);
    let plus = hits
        .rect_of(&Target::SetPreference(more))
        .expect("the terminal's +");
    let asked = click(plus, &mut ui, &app, &hits);
    assert_eq!(
        asked,
        [Command::Config(config::Command::SetPreference(more))]
    );
}

#[test]
fn opening_settings_checks_the_environment_again() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let footer = hits.rect_of(&Target::SettingsOpen).expect("the footer row");
    let asked = click(footer, &mut ui, &app, &hits);
    assert_eq!(asked, [Command::Config(config::Command::CheckEnvironment)]);
}

#[test]
fn setup_shows_each_program_in_its_state_and_offers_the_claude_sign_in() {
    let (mut app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Setup;
    let found = |version: &str, signed_in| {
        Some(Found {
            version: version.into(),
            signed_in,
        })
    };
    let tool = |name, required, found| Tool {
        name,
        purpose: "what it does",
        required,
        found,
    };
    app.config.tools = Some(vec![
        tool("git", true, found("2.43.0", None)),
        tool("claude", true, found("2.1.3", Some(false))),
        tool("glab", false, None),
    ]);
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "2 not ready",
        "2.43.0",
        "2.1.3 · not signed in",
        "not found · what it does",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let sign_in = hits
        .rect_of(&Target::SettingsLogin)
        .expect("claude's sign in");
    let asked = click(sign_in, &mut ui, &app, &hits);
    let login = config::Command::Login { cols: 80, rows: 24 };
    assert_eq!(asked, [Command::Config(login)]);
}

#[test]
fn providers_show_each_source_its_fields_and_what_each_gap_costs() {
    let (app, mut ui) = opened();
    let mut app = crate::tests::bar::sourced(app, true, true);
    if let Some(notion) = app.config.config.as_mut().and_then(|c| c.notion.as_mut()) {
        notion.properties.status = "Status".into();
        notion.properties.due = Some("Due date".into());
        notion.sprint = None;
    }
    ui.settings.section = crate::views::settings::Section::Providers;
    let tall = metrics(1280, 1600, 1.0);
    let (frame, _) = view(&app, &ui, tall, &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    for shown in [
        "Notion",
        "GitHub",
        "d",
        "github.com",
        "Status",
        "Due date",
        "from gh",
        "Assignee",
        "gap · no task is listed",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let gap = "gap · the hours measured are not logged";
    assert_eq!(
        texts.iter().filter(|t| *t == gap).count(),
        2,
        "one per source"
    );
    assert!(!texts.iter().any(|t| t == "t"), "the token is never shown");
}

#[test]
fn a_form_taller_than_the_window_scrolls_to_its_last_group() {
    let (app, mut ui) = opened();
    let app = crate::tests::bar::sourced(app, true, true);
    ui.settings.section = crate::views::settings::Section::Providers;
    let short = metrics(1280, 400, 1.0);
    let texts = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, short, &mut Fonts::embedded());
        let texts: Vec<String> = frame.layers()[0]
            .texts
            .iter()
            .map(|t| t.text.clone())
            .collect();
        (texts, hits)
    };
    let (before, hits) = texts(&ui);
    assert!(
        !before.iter().any(|t| t == "Forge tokens"),
        "below the fold"
    );
    let down = crate::input::Delta::Pixels {
        across: 0.0,
        down: -2000.0,
    };
    let wheel = crate::input::Input::Scroll {
        x: 900.0,
        y: 200.0,
        delta: down,
    };
    crate::input::handle(wheel, &mut ui, &app, &hits, short);
    assert!(ui.settings.scroll > 0.0);
    let (after, _) = texts(&ui);
    assert!(after.iter().any(|t| t == "Forge tokens"), "{after:?}");
}

fn typed(text: &str, ui: &mut Ui, app: &AppState) {
    for c in text.chars() {
        press(Key::Char(c), Modifiers::default(), ui, app);
    }
}

#[test]
fn a_source_turns_on_through_its_fields_and_the_token_never_shows() {
    let (app, mut ui) = opened();
    let app = crate::tests::bar::sourced(app, true, false);
    ui.settings.section = crate::views::settings::Section::Providers;
    let (_, hits) = drawn(&app, &ui);
    let on = Target::SettingsTurnOn(ProviderId::Notion);
    click(
        hits.rect_of(&on).expect("Notion's turn on"),
        &mut ui,
        &app,
        &hits,
    );
    typed("ntn_secret", &mut ui, &app);
    press(Key::Tab, Modifiers::default(), &mut ui, &app);
    typed("DB", &mut ui, &app);
    press(Key::Tab, Modifiers::default(), &mut ui, &app);
    typed("ME", &mut ui, &app);
    let (texts, _) = drawn(&app, &ui);
    assert!(!texts.iter().any(|t| t.contains("ntn_secret")), "{texts:?}");
    let asked = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let connect = config::Command::ConnectNotion {
        token: Secret::new("ntn_secret"),
        database_id: "DB".into(),
        user_id: "ME".into(),
    };
    assert_eq!(asked, [Command::Config(connect)]);
}

#[test]
fn a_source_turns_off_only_once_confirmed() {
    let (app, mut ui) = opened();
    let app = crate::tests::bar::sourced(app, true, true);
    ui.settings.section = crate::views::settings::Section::Providers;
    let (_, hits) = drawn(&app, &ui);
    let off = Target::SettingsTurnOff(ProviderId::Github);
    let asked = click(
        hits.rect_of(&off).expect("GitHub's turn off"),
        &mut ui,
        &app,
        &hits,
    );
    assert!(asked.is_empty(), "the first click only asks");
    let (_, hits) = drawn(&app, &ui);
    let sure = Target::SettingsTurnOffSure(ProviderId::Github);
    let asked = click(
        hits.rect_of(&sure).expect("the confirm"),
        &mut ui,
        &app,
        &hits,
    );
    let turn_off = config::Command::TurnOff(ProviderId::Github);
    assert_eq!(asked, [Command::Config(turn_off)]);
}

#[test]
fn the_palette_opens_settings_too() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('k'), ALT, &mut ui, &app);
    for c in "settings".chars() {
        press(Key::Char(c), Modifiers::default(), &mut ui, &app);
    }
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(ui.settings.open);
    assert!(ui.palette().is_none());
}

#[test]
fn the_shared_repo_is_named_through_its_url_and_branch() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Agent;
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|t| t == "Shared repo"), "{texts:?}");
    let set = hits
        .rect_of(&Target::SettingsShare)
        .expect("the repo's set");
    click(set, &mut ui, &app, &hits);
    typed("git@github.com:team/skills.git", &mut ui, &app);
    press(Key::Tab, Modifiers::default(), &mut ui, &app);
    typed("prod", &mut ui, &app);
    let asked = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let join = config::Command::JoinShared {
        url: "git@github.com:team/skills.git".into(),
        branch: "prod".into(),
    };
    assert_eq!(asked, [Command::Config(join)]);
}

#[test]
fn a_named_repo_says_its_plugins_and_goes_only_once_confirmed() {
    let (app, mut ui) = opened();
    let mut app = crate::tests::bar::sourced(app, false, false);
    let shared = groove_types::SharedConfig {
        url: "git@github.com:team/skills.git".into(),
        branch: "main".into(),
        enabled: Vec::new(),
    };
    app.config.config.as_mut().expect("a config").shared = Some(shared);
    let plugin = |name: &str| groove_controllers::agent_service::skills::Plugin {
        name: name.into(),
        dir: format!("/data/shared/plugins/{name}").into(),
    };
    app.agent.shared = Some(groove_controllers::agent_service::shared::Shared {
        path: "/data/shared".into(),
        marketplace: groove_controllers::agent_service::skills::Marketplace {
            name: "groove-agent".into(),
            plugins: vec![plugin("platform"), plugin("review")],
        },
    });
    ui.settings.section = crate::views::settings::Section::Agent;
    let (texts, hits) = drawn(&app, &ui);
    let named = "groove-agent · platform, review";
    for shown in [named, "git@github.com:team/skills.git", "main"] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let stop = hits.rect_of(&Target::SettingsUnshare).expect("the stop");
    assert!(
        click(stop, &mut ui, &app, &hits).is_empty(),
        "the first click only asks"
    );
    let (_, hits) = drawn(&app, &ui);
    let sure = hits
        .rect_of(&Target::SettingsUnshareSure)
        .expect("the confirm");
    let asked = click(sure, &mut ui, &app, &hits);
    assert_eq!(asked, [Command::Config(config::Command::LeaveShared)]);
}

#[test]
fn the_auto_approve_default_stands_in_the_agent_section() {
    let (app, mut ui) = opened();
    let app = crate::tests::bar::sourced(app, false, false);
    let shown = |ui: &Ui| {
        drawn(&app, ui)
            .0
            .iter()
            .any(|t| t == "auto-approve default")
    };
    ui.settings.section = crate::views::settings::Section::Agent;
    assert!(shown(&ui));
    ui.settings.section = crate::views::settings::Section::Preferences;
    assert!(!shown(&ui), "Preferences no longer holds it");
}

fn skill(id: &str, enabled: bool) -> groove_types::Skill {
    let (plugin, name) = id.split_once(':').expect("an id");
    groove_types::Skill {
        id: id.into(),
        plugin: plugin.into(),
        name: name.into(),
        description: format!("{name} things"),
        hint: String::new(),
        label: name.into(),
        kinds: Vec::new(),
        editable: plugin == "user",
        enabled,
        changed_at: groove_types::Timestamp::new(0),
    }
}

/// Settings › Agent over a core skill, one of the user's own, and a shared one switched off.
fn skilled() -> (AppState, Ui) {
    let (app, mut ui) = opened();
    let mut app = crate::tests::bar::sourced(app, false, false);
    app.agent.skills = vec![
        skill("groove:save-task", true),
        skill("platform:follow-ups", false),
        skill("user:ship-it", true),
    ];
    app.agent.shared = Some(groove_controllers::agent_service::shared::Shared {
        path: "/data/shared".into(),
        marketplace: groove_controllers::agent_service::skills::Marketplace {
            name: "groove-agent".into(),
            plugins: Vec::new(),
        },
    });
    ui.settings.section = crate::views::settings::Section::Agent;
    (app, ui)
}

#[test]
fn every_skill_stands_in_its_group_the_core_ones_always_on() {
    let (app, ui) = skilled();
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "Core skills",
        "Your skills",
        "Shared skills",
        "groove:save-task",
        "always",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let switch = |id: &str, on| Target::SkillSwitch(id.into(), on);
    assert!(
        hits.rect_of(&switch("groove:save-task", false)).is_none(),
        "no switch on a core one"
    );
    assert!(
        hits.rect_of(&switch("platform:follow-ups", true)).is_some(),
        "off, so on is offered"
    );
    assert!(
        hits.rect_of(&Target::SkillDelete("platform:follow-ups".into()))
            .is_none()
    );
}

#[test]
fn a_skill_of_the_users_is_switched_and_deleted_once_confirmed() {
    let (app, mut ui) = skilled();
    let (_, hits) = drawn(&app, &ui);
    let off = Target::SkillSwitch("user:ship-it".into(), false);
    let asked = click(
        hits.rect_of(&off).expect("its switch"),
        &mut ui,
        &app,
        &hits,
    );
    let switch = groove_controllers::agent::Command::SwitchSkill {
        id: "user:ship-it".into(),
        on: false,
    };
    assert_eq!(asked, [Command::Agent(switch)]);

    let delete = Target::SkillDelete("user:ship-it".into());
    let asked = click(
        hits.rect_of(&delete).expect("its delete"),
        &mut ui,
        &app,
        &hits,
    );
    assert!(asked.is_empty(), "the first click only asks");
    let (_, hits) = drawn(&app, &ui);
    let sure = Target::SkillDeleteSure("user:ship-it".into());
    let asked = click(
        hits.rect_of(&sure).expect("the confirm"),
        &mut ui,
        &app,
        &hits,
    );
    let gone = groove_controllers::agent::Command::DeleteSkill {
        name: "ship-it".into(),
    };
    assert_eq!(asked, [Command::Agent(gone)]);
}
