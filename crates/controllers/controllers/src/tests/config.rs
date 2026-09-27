//! Preferences: a change reaches its readers and the file at once, and new sessions start from it.

use groove_config_service::Preference;
use groove_types::ThemeName;

use crate::config::Command;
use crate::session::Command as SessionCommand;
use crate::{Command as Cmd, dispatch};

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
fn a_new_session_takes_the_auto_approve_default() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let set = Command::SetPreference(Preference::AutoApproveDefault(true));
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    let explorer = SessionCommand::OpenExplorer { title: None };
    dispatch(Cmd::Session(explorer), &mut state, &services, &spawner);
    let open = state.session.selected().expect("the explorer is open");
    assert!(open.state.auto_approve);
}
