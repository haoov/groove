//! A work clone pushed to a bare origin: `main` and `release/1.0`, `origin/HEAD` on main,
//! one unpushed commit on top.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Git;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub origin: PathBuf,
    pub work: PathBuf,
}

/// Plain git for setting a scene, with a repo-local identity and no signing.
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
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("tempdir");
        let origin = root.path().join("origin.git");
        let work = root.path().join("work");
        std::fs::create_dir_all(&origin).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        sh(&origin, &["init", "--bare", "--initial-branch=main", "."]);
        sh(&work, &["init", "--initial-branch=main", "."]);
        sh(&work, &["config", "user.email", "t@t"]);
        sh(&work, &["config", "user.name", "T"]);
        sh(&work, &["config", "commit.gpgsign", "false"]);
        std::fs::write(work.join("a.txt"), "one\n").unwrap();
        sh(&work, &["add", "."]);
        sh(&work, &["commit", "-m", "first"]);
        sh(
            &work,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        sh(&work, &["push", "origin", "main"]);
        sh(&work, &["push", "origin", "main:release/1.0"]);
        sh(&work, &["fetch", "origin"]);
        sh(&work, &["remote", "set-head", "origin", "main"]);
        std::fs::write(work.join("a.txt"), "two\n").unwrap();
        sh(&work, &["commit", "-am", "second"]);
        Self { root, origin, work }
    }

    pub fn git(&self) -> Git {
        Git::at(&self.work)
    }
}
