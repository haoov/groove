//! A fake `claude` and the state it runs in, shared by the controller tests.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{AppState, Env, Services, SyncSpawner};

/// Prints its first flag; echoes one line, or its cwd when the line is `pwd`; exits 7.
const FAKE_CLAUDE: &str = "#!/bin/sh
printf 'ready %s\\n' \"$1\"
read line
if [ \"$line\" = pwd ]; then line=$(pwd); fi
printf 'got %s\\n' \"$line\"
exit 7
";

/// An `AppState` whose `<home>/.local/bin/claude` is the fake.
pub fn state(home: &Path) -> AppState {
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let claude = bin.join("claude");
    std::fs::write(&claude, FAKE_CLAUDE).unwrap();
    std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut state = AppState::new(Env {
        home: home.to_path_buf(),
        config_dir: home.join("config"),
        data_dir: home.join("data"),
        plugin_dirs: vec![],
    });
    let config: groove_types::Config =
        serde_json::from_str(r#"{ "git": { "worktree_root": "~/code" } }"#).unwrap();
    state.config.config = Some(config);
    std::fs::create_dir_all(home.join("code")).unwrap();
    state
}

/// Every service on an in-memory database, the pool under `<home>/code`.
pub fn services(spawner: &SyncSpawner, home: &Path) -> Services {
    spawner
        .block_on(Services::in_memory(&home.join("code")))
        .unwrap()
}

/// A bare origin and a pooled clone of it at `<home>/code/main/gitlab.example.com/g/mayo`.
pub fn pooled_clone(home: &Path) -> PathBuf {
    let origin = home.join("origin.git");
    let seed = home.join("seed");
    std::fs::create_dir_all(&origin).unwrap();
    std::fs::create_dir_all(&seed).unwrap();
    sh(&origin, &["init", "--bare", "--initial-branch=main", "."]);
    sh(&seed, &["init", "--initial-branch=main", "."]);
    std::fs::write(seed.join("a.txt"), "one\n").unwrap();
    sh(&seed, &["add", "."]);
    sh(&seed, &["commit", "-m", "first"]);
    sh(
        &seed,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    sh(&seed, &["push", "origin", "main"]);
    sh(&seed, &["push", "origin", "main:release/1.0"]);
    let clone = home.join("code/main/gitlab.example.com/g/mayo");
    std::fs::create_dir_all(clone.parent().unwrap()).unwrap();
    sh(
        home,
        &["clone", origin.to_str().unwrap(), clone.to_str().unwrap()],
    );
    clone
}

pub fn sh(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=T",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .env("LC_ALL", "C")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Drains continuations until `done`, or fails after ten seconds.
pub fn until(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    mut done: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        spawner.drain(state, services);
        if done(state) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}
