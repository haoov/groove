use groove_types::{Config, GitConfig, UiConfig};

use crate::State;

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
