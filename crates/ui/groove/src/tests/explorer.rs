use groove_controllers::{AppState, Env, Services, SyncSpawner, dispatch, session};
use groove_types::WorktreeSpec;
use groove_ui::input::{Input, handle};
use groove_ui::{Metrics, Tokens, Ui, view};

/// A window's worth of metrics, as the app builds them.
pub(super) fn window() -> Metrics {
    let design = Tokens::new(1.0);
    Metrics {
        size: groove_gfx::Size::new(1600, 900),
        scale: 1.0,
        text: design.text,
        code: design.code,
        terminal: design.code,
        cell: groove_gfx::CellSize {
            width: 8.0,
            height: 17.0,
        },
        advance: 8.0,
        tick: 0,
        now: groove_types::Timestamp::now(),
    }
}

/// A bare origin, a pooled clone of it, and a session holding a worktree of it.
pub(super) fn working(home: &std::path::Path, spawner: &SyncSpawner) -> (AppState, Services) {
    let origin = home.join("origin.git");
    let seed = home.join("seed");
    for dir in [&origin, &seed] {
        std::fs::create_dir_all(dir).unwrap();
    }
    sh(&origin, &["init", "--bare", "--initial-branch=main", "."]);
    sh(&seed, &["init", "--initial-branch=main", "."]);
    std::fs::create_dir_all(seed.join("src/one")).unwrap();
    std::fs::write(seed.join("src/one/alpha.rs"), "fn one() {}\n").unwrap();
    std::fs::write(seed.join("README.md"), "read me\n").unwrap();
    sh(&seed, &["add", "."]);
    sh(&seed, &["commit", "-m", "first"]);
    sh(
        &seed,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    sh(&seed, &["push", "origin", "main"]);
    let clone = home.join("code/main/gitlab.example.com/g/mayo");
    std::fs::create_dir_all(clone.parent().unwrap()).unwrap();
    sh(
        home,
        &["clone", origin.to_str().unwrap(), clone.to_str().unwrap()],
    );

    let services = spawner
        .block_on(Services::in_memory(&home.join("code")))
        .unwrap();
    let mut state = AppState::new(Env {
        home: home.to_path_buf(),
        config_dir: home.join("config"),
        data_dir: home.join("data"),
        hooks: None,
        tools: None,
        shell: "/bin/sh".into(),
        kubeconfig: Vec::new(),
    });
    let config: groove_types::Config =
        serde_json::from_str(r#"{ "git": { "worktree_root": "~/code" } }"#).unwrap();
    state.config.config = Some(config);
    state.focused = true;
    dispatch(
        groove_controllers::Command::Session(session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        spawner,
    );
    let id = state.session.selected.clone().expect("a session");
    dispatch(
        groove_controllers::Command::Session(session::Command::AddRepo {
            session: id,
            name: "mayo".into(),
            spec: WorktreeSpec::default(),
        }),
        &mut state,
        &services,
        spawner,
    );
    until(spawner, &services, &mut state, |s| {
        s.workspace.worktree.is_some() || !s.errors.is_empty()
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    (state, services)
}

pub(super) fn sh(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

#[track_caller]
fn until(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    mut done: impl FnMut(&AppState) -> bool,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        spawner.drain(state, services);
        if done(state) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out; it failed: {:?}",
            state.errors
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn picking_the_files_tab_fills_the_tree_it_draws() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = working(home.path(), &spawner);
    let mut ui = Ui::default();
    ui.session.tab = groove_ui::Tab::Diff;

    let (_, hits) = view(&state, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let rect = hits
        .rect_of(&groove_ui::Target::Tab(groove_ui::Tab::Files))
        .expect("the tab is on screen");
    handle(
        Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &state,
        &hits,
        window(),
    );
    assert_eq!(ui.session.tab, groove_ui::Tab::Files);

    let asked = groove_ui::frame_commands(&state, &ui, window());
    assert!(
        asked.iter().any(|one| one.id() == "workspace.list_paths"),
        "the frame asks for the walk the tree needs: {:?}",
        asked.iter().map(|one| one.id()).collect::<Vec<_>>()
    );
    for command in asked {
        dispatch(command, &mut state, &services, &spawner);
    }
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.paths().is_empty()
    });

    let (frame, _) = view(&state, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    assert!(
        !texts
            .iter()
            .any(|one| one.starts_with("reading the worktree")),
        "the tree is drawn, not a promise of one: {texts:?}"
    );
    assert!(texts.iter().any(|one| one == "src"), "{texts:?}");
    assert!(texts.iter().any(|one| one == "README.md"), "{texts:?}");
}

#[test]
fn a_session_with_no_worktree_says_there_is_none_to_read() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = spawner
        .block_on(Services::in_memory(&home.path().join("code")))
        .unwrap();
    let mut state = AppState::new(Env {
        home: home.path().to_path_buf(),
        config_dir: home.path().join("config"),
        data_dir: home.path().join("data"),
        hooks: None,
        tools: None,
        shell: "/bin/sh".into(),
        kubeconfig: Vec::new(),
    });
    dispatch(
        groove_controllers::Command::Session(session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    let mut ui = Ui::default();
    ui.session.tab = groove_ui::Tab::Diff;
    ui.session.tab = groove_ui::Tab::Files;

    assert!(
        !groove_ui::frame_commands(&state, &ui, window())
            .iter()
            .any(|one| one.id() == "workspace.list_paths"),
        "with no worktree there is nothing to walk, so nothing is asked"
    );
    let (frame, _) = view(&state, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    assert!(
        texts.iter().any(|one| one == "no worktree to read"),
        "it says what is true: {texts:?}"
    );
    assert!(
        !texts.iter().any(|one| one.starts_with("reading")),
        "and never claims to be reading: {texts:?}"
    );
}
