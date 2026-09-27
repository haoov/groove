use groove_types::{Config, GitConfig, UiConfig};

use crate::{Preference, State};

fn with(font_size: f32, code_font_size: f32) -> State {
    let config = Config {
        notion: None,
        github: None,
        git: GitConfig {
            worktree_root: "~/worktrees".into(),
        },
        ui: UiConfig {
            font_size,
            code_font_size,
            ..UiConfig::default()
        },
        preferences: groove_types::Preferences::default(),
    };
    State {
        config: Some(config),
    }
}

#[test]
fn the_sizes_are_the_design_s_own_before_the_file_says_otherwise() {
    let state = State::default();
    assert_eq!(state.text_size(), 13.0);
    assert_eq!(state.code_size(), 12.5);
}

#[test]
fn the_file_sets_each_size_on_its_own() {
    let state = with(16.0, 14.0);
    assert_eq!(state.text_size(), 16.0);
    assert_eq!(state.code_size(), 14.0);
}

#[test]
fn a_size_no_font_can_read_as_is_refused() {
    for (text, code) in [(0.0, 0.0), (400.0, 400.0), (-12.0, 7.0)] {
        let state = with(text, code);
        assert_eq!(state.text_size(), 13.0, "{text} is not a size");
        assert_eq!(state.code_size(), 12.5, "{code} is not a size");
    }
}

#[test]
fn a_preference_changed_is_what_its_readers_answer_after() {
    let mut state = with(13.0, 12.5);
    assert!(state.set(Preference::PollIntervalSecs(120)).is_some());
    assert!(state.set(Preference::AutoApproveDefault(true)).is_some());
    assert!(state.set(Preference::DueSoonDays(5)).is_some());
    assert_eq!(state.poll_interval(), 120);
    assert!(state.auto_approve_default());
    assert_eq!(state.thresholds().due_soon_days, 5);
}

#[test]
fn the_poll_never_runs_faster_than_its_floor() {
    let mut state = with(13.0, 12.5);
    state.set(Preference::PollIntervalSecs(1));
    assert_eq!(state.poll_interval(), 10);
}

#[test]
fn nothing_is_set_before_the_first_run_wrote_a_config() {
    let mut state = State::default();
    assert!(state.set(Preference::StaleAfterSecs(30)).is_none());
}
