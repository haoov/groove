//! A fake `claude` and the state it runs in, shared by the controller tests.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use groove_types::WorktreeSpec;

use crate::{
    AppState, Command as Cmd, Env, Services, SyncSpawner, config, delivery, dispatch, session,
    workspace,
};

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
        hooks: None,
        tools: None,
        shell: "/bin/sh".into(),
        kubeconfig: vec![home.join(".kube/config")],
        cache_dir: home.join("cache"),
    });
    let config: groove_types::Config =
        serde_json::from_str(r#"{ "git": { "worktree_root": "~/code" } }"#).unwrap();
    state.config.config = Some(config);
    std::fs::create_dir_all(home.join("code")).unwrap();
    state
}

/// A home of its own with the fake `claude`, the services on it, and the state.
pub fn fresh() -> (tempfile::TempDir, SyncSpawner, Services, AppState) {
    let home = home();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let state = state(home.path());
    (home, spawner, services, state)
}

/// A temp directory by its real path: macOS's `/var` is a link a child's `pwd` resolves.
pub fn home() -> tempfile::TempDir {
    let tmp = std::env::temp_dir().canonicalize().unwrap();
    tempfile::tempdir_in(tmp).unwrap()
}

/// Every service on an in-memory database, the pool under `<home>/code`.
pub fn services(spawner: &SyncSpawner, home: &Path) -> Services {
    spawner
        .block_on(Services::in_memory(&home.join("code")))
        .unwrap()
}

/// `<home>/.kube/config` with one context, `kind`, on `server`.
pub fn kubeconfig(home: &Path, server: &str) {
    let yaml = format!(
        "apiVersion: v1
kind: Config
clusters:
- {{name: kind, cluster: {{server: '{server}'}}}}
users:
- {{name: kind, user: {{token: not-a-real-token}}}}
contexts:
- {{name: kind, context: {{cluster: kind, user: kind}}}}
"
    );
    std::fs::create_dir_all(home.join(".kube")).unwrap();
    std::fs::write(home.join(".kube/config"), yaml).unwrap();
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
    for (key, value) in [
        ("user.email", "t@t"),
        ("user.name", "T"),
        ("commit.gpgsign", "false"),
    ] {
        sh(&clone, &["config", key, value]);
    }
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

/// An explorer open, its agent started; returns its id.
pub fn explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> groove_types::SessionId {
    dispatch(
        Cmd::Session(session::Command::OpenExplorer {
            title: Some("try mayo".into()),
        }),
        state,
        services,
        spawner,
    );
    let id = state.session.selected.clone().expect("a session");
    until(spawner, services, state, |s| s.agent.agent(&id).is_some());
    id
}

/// An explorer with the fixture's repo and one worktree; returns the worktree's path.
pub fn worktree(state: &mut AppState, services: &Services, spawner: &SyncSpawner) -> String {
    let id = explorer(state, services, spawner);
    dispatch(
        Cmd::Session(session::Command::AddRepo {
            session: id.clone(),
            name: "mayo".into(),
            spec: WorktreeSpec::default(),
        }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |s| {
        s.workspace.worktree.is_some() || !s.errors.is_empty()
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    state
        .session
        .get(&id)
        .and_then(|o| o.selected_worktree())
        .map(|w| w.path.clone())
        .expect("a worktree")
}

/// A command `send` takes: the app's own, or one controller's.
pub trait Routed {
    fn routed(self) -> Cmd;
}

impl Routed for Cmd {
    fn routed(self) -> Cmd {
        self
    }
}

impl Routed for workspace::Command {
    fn routed(self) -> Cmd {
        Cmd::Workspace(self)
    }
}

impl Routed for delivery::Command {
    fn routed(self) -> Cmd {
        Cmd::Delivery(self)
    }
}

impl Routed for config::Command {
    fn routed(self) -> Cmd {
        Cmd::Config(self)
    }
}

/// One command, run to the end of everything it starts.
#[track_caller]
pub fn send(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: impl Routed,
) {
    dispatch(command.routed(), state, services, spawner);
    until(spawner, services, state, |s| {
        s.pending.is_empty() && spawner.idle()
    });
}

#[track_caller]
pub fn until(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    mut done: impl FnMut(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        spawner.drain(state, services);
        if done(state) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out; it failed: {:?}",
            state.errors
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The session's writes run without asking.
pub fn auto_approve(state: &mut AppState, session: &groove_types::SessionId) {
    if let Some(open) = state.session.get_mut(session) {
        open.state.auto_approve = true;
    }
}

/// Holds every job, to run them in any order; a thread reports through `sink` when given one.
#[derive(Default)]
pub struct Held {
    jobs: std::sync::Mutex<Vec<crate::spawn::Job>>,
    sink: Option<std::sync::Arc<dyn crate::Deliver>>,
}

impl Held {
    /// Jobs held, and what a thread reports delivered to `spawner` for its `drain`.
    pub fn over(spawner: &SyncSpawner) -> Self {
        let sink = Some(crate::Spawner::sink(spawner));
        Self {
            jobs: Default::default(),
            sink,
        }
    }

    /// The jobs held so far, in the order they were spawned.
    pub fn take(&self) -> Vec<crate::spawn::Job> {
        std::mem::take(&mut *self.jobs.lock().expect("the jobs"))
    }

    /// The held jobs started newest first on a pool, a beat apart; their continuations applied after.
    pub fn backwards(&self, state: &mut AppState, services: &Services) {
        let pool = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let mut out = Vec::new();
        for job in self.take().into_iter().rev() {
            out.push(pool.spawn(job));
            std::thread::sleep(Duration::from_millis(100));
        }
        let spawner = SyncSpawner::new().unwrap();
        for one in out {
            pool.block_on(one).expect("the job")(state, services, &spawner);
        }
    }
}

impl crate::Spawner for Held {
    fn spawn(&self, job: crate::spawn::Job) {
        self.jobs.lock().expect("the jobs").push(job);
    }

    fn detach(&self, _: crate::Detached) {}

    fn sink(&self) -> std::sync::Arc<dyn crate::Deliver> {
        self.sink.clone().expect("a held spawner made `over` one")
    }
}
