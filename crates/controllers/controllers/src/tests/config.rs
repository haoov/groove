//! Preferences: a change reaches its readers and the file at once, and new sessions start from it.

use groove_config_service::Preference;
use groove_types::{FontFamily, ProviderId, Secret, ThemeName};

use crate::config::Command;
use crate::session::Command as SessionCommand;
use crate::{Command as Cmd, dispatch};

#[test]
fn the_environment_check_lists_every_program_and_one_runs_at_a_time() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Config(Command::CheckEnvironment),
        &mut state,
        &services,
        &spawner,
    );
    assert!(state.config.checking);
    dispatch(
        Cmd::Config(Command::CheckEnvironment),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(!state.config.checking);
    let tools = state.config.tools.as_ref().expect("the check ended");
    let names: Vec<_> = tools.iter().map(|one| one.name).collect();
    assert_eq!(names, ["git", "claude", "curl", "glab", "gh"]);
    assert!(tools[0].found.is_some(), "git runs where the tests run");
}

#[test]
fn a_source_asked_without_what_it_needs_or_the_last_one_turned_off_is_refused() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let connect = Command::ConnectNotion {
        token: Secret::new("ntn_secret"),
        database_id: "DB".into(),
        user_id: " ".into(),
    };
    dispatch(Cmd::Config(connect), &mut state, &services, &spawner);
    assert!(state.config.connecting.is_none(), "nothing was read");
    let why = state
        .config
        .refused
        .clone()
        .expect("the refusal is kept for Setup");
    assert!(
        why.contains("user id"),
        "only your tasks, so the user is needed: {why}"
    );
    let off = Command::TurnOff(ProviderId::Github);
    dispatch(Cmd::Config(off), &mut state, &services, &spawner);
    assert!(state.config.refused.is_some());
}

#[test]
fn a_preference_set_is_read_at_once_and_written_to_the_file() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let set = Command::SetPreference(Preference::PollIntervalSecs(120));
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    assert_eq!(
        state.config.poll_interval(),
        120,
        "every reader sees it now"
    );
    let written = groove_config_service::load(&home.path().join("config")).unwrap();
    let written = written.expect("the file is there");
    assert_eq!(written.preferences.poll_interval_secs, 120);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_theme_picked_is_the_one_drawn_and_the_one_written() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let set = Command::SetPreference(Preference::Theme(ThemeName::Mocha));
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    assert_eq!(state.config.theme(), ThemeName::Mocha);
    let written = groove_config_service::load(&home.path().join("config")).unwrap();
    assert_eq!(
        written.expect("the file is there").ui.theme,
        ThemeName::Mocha
    );
}

#[test]
fn a_mono_family_picked_is_read_and_written() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let set = Command::SetPreference(Preference::MonoFamily(FontFamily::Lilex));
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    assert_eq!(state.config.mono_family(), FontFamily::Lilex);
    let written = groove_config_service::load(&home.path().join("config")).unwrap();
    let ui = written.expect("the file is there").ui;
    assert_eq!(ui.mono_font_family, "Lilex");
}

#[test]
fn a_new_session_takes_the_auto_approve_default() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let set = Command::SetPreference(Preference::AutoApproveDefault(true));
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    let explorer = SessionCommand::OpenExplorer { title: None };
    dispatch(Cmd::Session(explorer), &mut state, &services, &spawner);
    let open = state.session.selected().expect("the explorer is open");
    assert!(open.state.auto_approve);
}
