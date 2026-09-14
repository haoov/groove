//! A root with one pooled clone of a bare origin, and a database holding one explorer.

use std::path::{Path, PathBuf};
use std::process::Command;

use groove_db::Db;
use groove_types::{Session, SessionId, SessionKind, Timestamp};

use crate::Pool;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub origin: PathBuf,
    pub clone: PathBuf,
    pub pool: Pool,
    pub session: Session,
}

pub const SLUG: &str = "gitlab.example.com/wiremind/devops/mayo";

pub fn sh(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
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

impl Fixture {
    pub async fn new() -> Self {
        let root = tempfile::tempdir().expect("tempdir");
        let origin = root.path().join("origin.git");
        std::fs::create_dir_all(&origin).unwrap();
        sh(&origin, &["init", "--bare", "--initial-branch=main", "."]);

        let seed = root.path().join("seed");
        std::fs::create_dir_all(&seed).unwrap();
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

        let clone = root.path().join("main").join(SLUG);
        std::fs::create_dir_all(clone.parent().unwrap()).unwrap();
        sh(
            root.path(),
            &["clone", origin.to_str().unwrap(), clone.to_str().unwrap()],
        );
        sh(&clone, &["config", "user.email", "t@t"]);
        sh(&clone, &["config", "user.name", "T"]);
        sh(&clone, &["config", "commit.gpgsign", "false"]);

        let db = Db::in_memory().await.unwrap();
        let session = Session {
            id: SessionId::new("explorer-ab12cd34"),
            title: "Try sqlite vacuum".into(),
            kind: SessionKind::Explorer,
            created_at: Timestamp::new(0),
        };
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, created_at) VALUES (?, 'explorer', ?, 0)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .execute(db.pool())
        .await
        .unwrap();
        let pool = Pool::new(db, root.path());
        Self {
            root,
            origin,
            clone,
            pool,
            session,
        }
    }

    /// The pooled clone, registered.
    pub async fn repo(&self) -> groove_types::Repo {
        let clones = self.pool.list();
        let clone = Pool::resolve("mayo", &clones).unwrap();
        self.pool.register(clone).await.unwrap()
    }
}
