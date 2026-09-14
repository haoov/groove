//! A fake `claude` and the state it runs in, shared by the controller tests.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{AppState, Env, SyncSpawner};

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
    AppState::new(Env {
        home: home.to_path_buf(),
        config_dir: home.join("config"),
        data_dir: home.join("data"),
        plugin_dirs: vec![],
    })
}

/// Drains continuations until `done`, or fails after ten seconds.
pub fn until(spawner: &SyncSpawner, state: &mut AppState, mut done: impl FnMut(&AppState) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        spawner.drain(state);
        if done(state) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}
