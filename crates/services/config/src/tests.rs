use groove_types::{ClusterChange, Config, GitConfig, Hue, UiConfig};

use crate::{Font, Preference, Source, State};

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
        keymap: Default::default(),
        shared: None,
        skills_off: Vec::new(),
        routines: Default::default(),
        clusters: Vec::new(),
    };
    State {
        config: Some(config),
        ..State::default()
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
fn the_terminal_takes_the_code_size_until_it_has_its_own() {
    let mut state = with(13.0, 17.0);
    assert_eq!(state.terminal_size(), 17.0);
    state.set(Preference::FontSize(Font::Terminal, 11.0));
    assert_eq!(
        (state.text_size(), state.code_size(), state.terminal_size()),
        (13.0, 17.0, 11.0)
    );
    state.set(Preference::FontSize(Font::Interface, 90.0));
    assert_eq!(
        state.text_size(),
        32.0,
        "a step past the largest stops at it"
    );
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
fn a_source_turns_on_and_off_but_the_last_one_stays() {
    let mut state = with(13.0, 12.5);
    let github = groove_types::GithubConfig::bare("github.com");
    assert!(state.set_source(Source::Github(Some(github))).is_ok());
    let refused = state.set_source(Source::Github(None));
    assert!(refused.is_err(), "nothing left to read tasks from");
    assert!(
        state.config.as_ref().is_some_and(|c| c.github.is_some()),
        "a refused change leaves the config as it was"
    );
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

#[test]
fn a_context_is_added_once_changed_on_its_own_and_removed() {
    let mut state = with(13.0, 12.5);
    state.add_cluster("hub");
    state.add_cluster("staging");
    state.add_cluster("hub");
    assert_eq!(state.clusters().len(), 2);
    assert_eq!(
        state.cluster("staging").map(|one| one.hue),
        Some(Hue::Mauve)
    );
    let written = state.change_cluster("hub", ClusterChange::ReadOnly(true));
    assert!(written.is_some_and(|config| config.clusters[0].read_only));
    assert!(
        state
            .change_cluster("prod", ClusterChange::ReadOnly(true))
            .is_none()
    );
    state.remove_cluster("hub");
    let left: Vec<_> = state
        .clusters()
        .iter()
        .map(|one| one.context.as_str())
        .collect();
    assert_eq!(left, ["staging"]);
}

#[tokio::test]
async fn an_older_config_held_after_a_newer_one_never_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let keeper = crate::Keeper::default();
    keeper.hold(dir.path(), with(14.0, 13.0).config.unwrap(), 2);
    keeper.hold(dir.path(), with(12.0, 13.0).config.unwrap(), 1);
    keeper.flush().await.unwrap();
    let kept = crate::load(dir.path()).unwrap().expect("the file is there");
    assert_eq!((kept.ui.font_size, keeper.kept()), (14.0, 2));
}

#[tokio::test]
async fn what_is_held_reaches_the_disk_on_a_flush_with_no_job_left() {
    let dir = tempfile::tempdir().unwrap();
    let keeper = crate::Keeper::default();
    let panes = groove_types::Panes {
        rail: 260.0,
        agent: 600.0,
        sidebar: 380.0,
        commit: 120.0,
        feed: 200.0,
        manual: 220.0,
    };
    keeper.hold(dir.path(), with(15.0, 13.0).config.unwrap(), 1);
    keeper.hold_panes(dir.path(), panes);
    keeper.flush().await.unwrap();
    let kept = crate::load(dir.path())
        .unwrap()
        .expect("the config is there");
    assert_eq!(kept.ui.font_size, 15.0);
    let file = groove_config::panes::path(dir.path());
    assert_eq!(groove_config::panes::load(&file).unwrap(), Some(panes));
}

#[tokio::test]
async fn after_a_failed_write_an_older_config_is_never_written() {
    let dir = tempfile::tempdir().unwrap();
    let keeper = crate::Keeper::default();
    let blocker = dir.path().join("config.json.tmp");
    std::fs::create_dir(&blocker).unwrap();
    keeper.hold(dir.path(), with(14.0, 13.0).config.unwrap(), 2);
    assert!(keeper.flush().await.is_err(), "the failure is said");
    std::fs::remove_dir(&blocker).unwrap();
    keeper.hold(dir.path(), with(12.0, 13.0).config.unwrap(), 1);
    keeper.flush().await.unwrap();
    let kept = crate::load(dir.path())
        .unwrap()
        .expect("the failed one, tried again");
    assert_eq!((kept.ui.font_size, keeper.kept()), (14.0, 2));
}

#[tokio::test]
async fn a_failed_write_is_tried_again_unless_a_newer_one_is_held() {
    let dir = tempfile::tempdir().unwrap();
    let keeper = crate::Keeper::default();
    let blocker = dir.path().join("config.json.tmp");
    std::fs::create_dir(&blocker).unwrap();
    keeper.hold(dir.path(), with(14.0, 13.0).config.unwrap(), 2);
    assert!(keeper.flush().await.is_err());
    std::fs::remove_dir(&blocker).unwrap();
    keeper.flush().await.unwrap();
    let kept = crate::load(dir.path()).unwrap().expect("tried again");
    assert_eq!((kept.ui.font_size, keeper.kept()), (14.0, 2));
    assert!(keeper.quit().await.is_ok(), "nothing is lost");

    std::fs::create_dir(&blocker).unwrap();
    keeper.hold(dir.path(), with(15.0, 13.0).config.unwrap(), 3);
    assert!(keeper.flush().await.is_err());
    keeper.hold(dir.path(), with(16.0, 13.0).config.unwrap(), 4);
    std::fs::remove_dir(&blocker).unwrap();
    keeper.flush().await.unwrap();
    let kept = crate::load(dir.path()).unwrap().expect("the newer one");
    assert_eq!((kept.ui.font_size, keeper.kept()), (16.0, 4));
}

#[tokio::test]
async fn a_write_that_failed_out_of_sight_fails_the_quit() {
    let dir = tempfile::tempdir().unwrap();
    let keeper = crate::Keeper::default();
    std::fs::create_dir(dir.path().join("config.json.tmp")).unwrap();
    keeper.hold(dir.path(), with(14.0, 13.0).config.unwrap(), 2);
    let _lost = keeper.flush().await;
    assert!(keeper.quit().await.is_err(), "the quit says it");
}

#[test]
fn a_skill_switched_before_the_first_run_says_there_is_no_config() {
    let mut state = State::default();
    let refused = state.switch_skill("user:mine", false).map(drop);
    let said = refused.expect_err("nothing to write").message;
    assert!(said.contains("first run"), "{said}");
}
