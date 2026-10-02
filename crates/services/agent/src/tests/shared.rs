//! The copy of the shared repo: made, moved to the branch's head, and refused when it is no marketplace.

use std::path::Path;
use std::process::Command;

use groove_types::SharedConfig;

use crate::shared::{follow, join};

fn sh(dir: &Path, args: &[&str]) {
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
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A bare origin whose `main` holds a marketplace of one plugin named `name`, and its clone.
fn origin(root: &Path, name: &str) -> (SharedConfig, std::path::PathBuf) {
    let (bare, work) = (root.join("origin.git"), root.join("work"));
    std::fs::create_dir_all(&bare).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    sh(&bare, &["init", "--bare", "--initial-branch=main", "."]);
    sh(&work, &["init", "--initial-branch=main", "."]);
    manifest(&work, name);
    sh(&work, &["remote", "add", "origin", bare.to_str().unwrap()]);
    sh(&work, &["push", "origin", "main"]);
    let url = format!("file://{}", bare.display());
    let branch = "main".to_string();
    let enabled = Vec::new();
    (
        SharedConfig {
            url,
            branch,
            enabled,
        },
        work,
    )
}

/// The marketplace listing one plugin, `name`, committed.
fn manifest(work: &Path, name: &str) {
    let (root, plugin) = (
        work.join(".claude-plugin"),
        work.join(name).join(".claude-plugin"),
    );
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&plugin).unwrap();
    let listed = serde_json::json!({
        "name": "team", "owner": {}, "plugins": [{ "name": name, "source": format!("./{name}") }]
    });
    std::fs::write(root.join("marketplace.json"), listed.to_string()).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        format!(r#"{{ "name": "{name}" }}"#),
    )
    .unwrap();
    sh(work, &["add", "."]);
    sh(work, &["commit", "-m", name]);
}

#[tokio::test]
async fn joining_copies_the_branch_and_names_its_plugin() {
    let root = tempfile::tempdir().unwrap();
    let (shared, _) = origin(root.path(), "wiremind");
    let data = root.path().join("data");
    let copy = join(&data, &shared).await.expect("a plugin");
    assert_eq!(names(&copy), ["wiremind"]);
    assert!(copy.path.starts_with(&data));
}

#[tokio::test]
async fn following_takes_what_the_branch_holds_now() {
    let root = tempfile::tempdir().unwrap();
    let (shared, work) = origin(root.path(), "wiremind");
    let data = root.path().join("data");
    join(&data, &shared).await.expect("a plugin");
    manifest(&work, "platform");
    sh(&work, &["push", "origin", "main"]);
    assert_eq!(
        names(&follow(&data, &shared).await.expect("followed")),
        ["platform"]
    );
}

#[tokio::test]
async fn a_repo_that_is_no_marketplace_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let (shared, _) = origin(root.path(), "groove");
    let data = root.path().join("data");
    let refused = join(&data, &shared).await.expect_err("groove is taken");
    assert!(refused.message.contains("groove"), "{}", refused.message);
}

#[tokio::test]
async fn an_http_url_that_cannot_be_read_says_to_give_the_ssh_one() {
    let root = tempfile::tempdir().unwrap();
    let shared = SharedConfig {
        url: "http://127.0.0.1:9/none.git".into(),
        branch: "main".into(),
        enabled: Vec::new(),
    };
    let refused = join(root.path(), &shared)
        .await
        .expect_err("nothing answers");
    assert!(refused.message.contains("SSH URL"), "{}", refused.message);
    assert!(
        !crate::shared::copy_of(root.path()).exists(),
        "no half copy is left"
    );
}

fn names(copy: &crate::shared::Shared) -> Vec<String> {
    copy.marketplace
        .plugins
        .iter()
        .map(|one| one.name.clone())
        .collect()
}
