//! Cached ref answers: `upstream_base` (the branch the work forks from) and
//! `diff_base` (the point a diff compares against).

use std::time::Duration;

use super::cache;
use super::run;

/// Fallbacks, in order, when a worktree has no pinned base.
const DEFAULT_CANDIDATES: [&str; 3] = ["origin/HEAD", "origin/main", "origin/master"];

/// Minimum interval between `git remote set-head` repairs per repo.
const SET_HEAD_THROTTLE: Duration = Duration::from_secs(60);

/// The upstream ref this worktree's work branches from. `pinned` is the MR
/// target branch (`Worktree::base_ref`), taken when it resolves on origin.
pub async fn upstream_base(path: &str, pinned: Option<&str>) -> anyhow::Result<String> {
    if let Some(b) = pinned.filter(|b| !b.is_empty()) {
        let target = format!("origin/{b}");
        if ref_exists(path, &target).await {
            return Ok(target);
        }
    }
    for candidate in DEFAULT_CANDIDATES {
        if ref_exists(path, candidate).await {
            return Ok(candidate.to_string());
        }
    }
    Err(anyhow::anyhow!(
        "no base branch on origin (tried {}{}) in {path} — is the remote fetched?",
        pinned
            .filter(|b| !b.is_empty())
            .map(|b| format!("origin/{b}, "))
            .unwrap_or_default(),
        DEFAULT_CANDIDATES.join(", "),
    ))
}

/// The revision a diff compares against. `working` = HEAD; `vs-remote` = the
/// branch's own remote tip, or the base when unpushed; anything else = the base.
/// Except for `working`, the answer is the merge base, not the base branch's tip.
pub async fn diff_base(
    path: &str,
    branch: &str,
    mode: &str,
    pinned: Option<&str>,
) -> anyhow::Result<String> {
    match mode {
        "working" => Ok("HEAD".to_string()),
        "vs-remote" => {
            let upstream = format!("origin/{branch}");
            let tip = if ref_exists(path, &upstream).await {
                upstream
            } else {
                upstream_base(path, pinned).await?
            };
            Ok(merge_base_or(path, &tip).await)
        }
        _ => {
            let tip = upstream_base(path, pinned).await?;
            Ok(merge_base_or(path, &tip).await)
        }
    }
}

/// `git merge-base <tip> HEAD`, or the tip itself when it cannot be computed.
async fn merge_base_or(path: &str, tip: &str) -> String {
    cache::shared()
        .text(format!("mb:{path}:{tip}"), || async {
            run::output(path, &["merge-base", tip, "HEAD"])
                .await
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .await
        .unwrap_or_else(|| tip.to_string())
}

pub async fn ref_exists(path: &str, git_ref: &str) -> bool {
    cache::shared()
        .flag(format!("ref:{path}:{git_ref}"), || async {
            run::output(path, &["rev-parse", "--verify", "--quiet", git_ref])
                .await
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .await
}

/// Every branch head on origin, asked of the remote itself; uncached.
/// `Err` means origin was unreachable, never "no such branch". Do not collapse the two.
pub async fn origin_branches(path: &str) -> anyhow::Result<Vec<String>> {
    let out = run::run(path, &["ls-remote", "--heads", "origin"]).await?;
    let mut branches: Vec<String> = out
        .lines()
        .filter_map(|l| l.split('\t').nth(1))
        .filter_map(|r| r.strip_prefix("refs/heads/"))
        .map(str::to_string)
        .collect();
    branches.sort();
    branches.dedup();
    Ok(branches)
}

/// The repo's default branch, from `refs/remotes/origin/HEAD`; a missing symref schedules a throttled repair.
/// Never strip the "origin/HEAD" shorthand: that yields "HEAD", and `fetch HEAD:HEAD` poisons a local branch.
pub async fn default_branch(repo_path: &str) -> Option<String> {
    cache::shared()
        .text(format!("db:{repo_path}"), || {
            resolve_default_branch(repo_path)
        })
        .await
}

async fn resolve_default_branch(repo_path: &str) -> Option<String> {
    async fn read_symref(repo_path: &str) -> Option<String> {
        let out = run::run(
            repo_path,
            &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        )
        .await
        .ok()?;
        match out.trim().strip_prefix("origin/") {
            Some(name) if !name.is_empty() && name != "HEAD" => Some(name.to_string()),
            _ => None,
        }
    }

    if let Some(name) = read_symref(repo_path).await {
        return Some(name);
    }

    // Repairing the symref talks to the remote; at most once a window per repo.
    if cache::shared().due(&format!("set-head:{repo_path}"), SET_HEAD_THROTTLE) {
        let _ = run::run(repo_path, &["remote", "set-head", "origin", "-a"]).await;
        if let Some(name) = read_symref(repo_path).await {
            return Some(name);
        }
    }

    for name in ["main", "master"] {
        if ref_exists(repo_path, &format!("refs/remotes/origin/{name}")).await {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn git(dir: &PathBuf, args: &[&str]) {
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

    impl Fixture {
        /// A work tree pushed to a bare origin, with `main` and `release/1.0`; `origin/HEAD` points at main.
        fn new(name: &str) -> (Self, String) {
            let root =
                std::env::temp_dir().join(format!("groove-refs-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            let fixture = Fixture { root: root.clone() };

            let origin = root.join("origin.git");
            std::fs::create_dir_all(&origin).unwrap();
            git(&origin, &["init", "--bare", "--initial-branch=main", "."]);

            let work = root.join("work");
            std::fs::create_dir_all(&work).unwrap();
            git(&work, &["init", "--initial-branch=main", "."]);
            std::fs::write(work.join("a.txt"), "one\n").unwrap();
            git(&work, &["add", "."]);
            git(&work, &["commit", "-m", "first"]);
            git(
                &work,
                &["remote", "add", "origin", origin.to_str().unwrap()],
            );
            git(&work, &["push", "origin", "main"]);
            git(&work, &["push", "origin", "main:release/1.0"]);
            git(&work, &["fetch", "origin"]);
            git(&work, &["remote", "set-head", "origin", "main"]);
            std::fs::write(work.join("a.txt"), "two\n").unwrap();
            git(&work, &["commit", "-am", "second"]);

            (fixture, work.to_str().unwrap().to_string())
        }
    }

    #[tokio::test]
    async fn prefers_the_pinned_target_branch() {
        let (_fx, work) = Fixture::new("pinned");
        let base = upstream_base(&work, Some("release/1.0")).await.unwrap();
        assert_eq!(base, "origin/release/1.0");
    }

    #[tokio::test]
    async fn falls_back_to_origin_head_when_the_pin_does_not_resolve() {
        let (_fx, work) = Fixture::new("badpin");
        let base = upstream_base(&work, Some("no-such-branch")).await.unwrap();
        assert_eq!(base, "origin/HEAD");
    }

    #[tokio::test]
    async fn ignores_an_empty_pin() {
        let (_fx, work) = Fixture::new("emptypin");
        assert_eq!(upstream_base(&work, Some("")).await.unwrap(), "origin/HEAD");
    }

    #[tokio::test]
    async fn falls_back_to_origin_main_when_the_remote_set_no_head() {
        let (_fx, work) = Fixture::new("nohead");
        let dir = PathBuf::from(&work);
        git(&dir, &["remote", "set-head", "origin", "--delete"]);
        assert_eq!(upstream_base(&work, None).await.unwrap(), "origin/main");
    }

    #[tokio::test]
    async fn errors_when_the_remote_has_no_base_at_all() {
        let (_fx, work) = Fixture::new("noremote");
        let dir = PathBuf::from(&work);
        git(&dir, &["remote", "remove", "origin"]);
        let err = upstream_base(&work, Some("main"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("origin/main"), "{err}");
        assert!(err.contains("no base branch"), "{err}");
    }

    #[tokio::test]
    async fn lists_every_branch_on_origin() {
        let (_fx, work) = Fixture::new("originlist");
        assert_eq!(
            origin_branches(&work).await.unwrap(),
            vec!["main", "release/1.0"]
        );
    }

    #[tokio::test]
    async fn lists_a_branch_no_local_ref_knows() {
        let (_fx, work) = Fixture::new("unfetched");
        let dir = PathBuf::from(&work);
        git(
            &dir,
            &["update-ref", "-d", "refs/remotes/origin/release/1.0"],
        );
        cache::flush();
        assert!(!ref_exists(&work, "origin/release/1.0").await);
        assert!(origin_branches(&work)
            .await
            .unwrap()
            .contains(&"release/1.0".to_string()));
    }

    #[tokio::test]
    async fn a_branch_deleted_on_origin_leaves_the_list() {
        let (_fx, work) = Fixture::new("deleted");
        let dir = PathBuf::from(&work);
        git(&dir, &["push", "origin", "--delete", "release/1.0"]);
        git(
            &dir,
            &["update-ref", "refs/remotes/origin/release/1.0", "HEAD"],
        );
        cache::flush();
        assert!(
            ref_exists(&work, "origin/release/1.0").await,
            "stale ref is the premise"
        );
        assert_eq!(origin_branches(&work).await.unwrap(), vec!["main"]);
    }

    #[tokio::test]
    async fn full_names_only_never_a_tail() {
        let (_fx, work) = Fixture::new("tail");
        let branches = origin_branches(&work).await.unwrap();
        assert!(branches.contains(&"release/1.0".to_string()));
        assert!(!branches.contains(&"1.0".to_string()));
    }

    #[tokio::test]
    async fn an_originless_clone_errors() {
        let (_fx, work) = Fixture::new("noorigin");
        let dir = PathBuf::from(&work);
        git(&dir, &["remote", "remove", "origin"]);
        assert!(origin_branches(&work).await.is_err());
    }

    #[tokio::test]
    async fn working_mode_diffs_against_head() {
        let (_fx, work) = Fixture::new("working");
        assert_eq!(
            diff_base(&work, "main", "working", None).await.unwrap(),
            "HEAD"
        );
    }

    #[tokio::test]
    async fn vs_main_resolves_to_the_merge_base() {
        let (_fx, work) = Fixture::new("mergebase");
        let dir = PathBuf::from(&work);
        let expected = Command::new("git")
            .args(["merge-base", "origin/HEAD", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let expected = String::from_utf8_lossy(&expected.stdout).trim().to_string();
        let got = diff_base(&work, "main", "vs-main", None).await.unwrap();
        assert_eq!(got, expected);
        assert_eq!(got.len(), 40, "a sha, not a ref name");
    }

    #[tokio::test]
    async fn vs_remote_uses_the_branch_own_remote_when_it_exists() {
        let (_fx, work) = Fixture::new("vsremote");
        let dir = PathBuf::from(&work);
        let expected = Command::new("git")
            .args(["merge-base", "origin/main", "HEAD"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let expected = String::from_utf8_lossy(&expected.stdout).trim().to_string();
        assert_eq!(
            diff_base(&work, "main", "vs-remote", None).await.unwrap(),
            expected
        );
    }

    #[tokio::test]
    async fn vs_remote_falls_back_to_the_base_for_an_unpushed_branch() {
        let (_fx, work) = Fixture::new("unpushed");
        let got = diff_base(&work, "feature/never-pushed", "vs-remote", None)
            .await
            .unwrap();
        assert_eq!(got.len(), 40);
    }

    #[tokio::test]
    async fn the_pinned_target_reaches_every_surface() {
        let (_fx, work) = Fixture::new("agree");
        let pinned = Some("release/1.0");
        let base = upstream_base(&work, pinned).await.unwrap();
        let diff = diff_base(&work, "main", "vs-main", pinned).await.unwrap();
        let expected = Command::new("git")
            .args(["merge-base", &base, "HEAD"])
            .current_dir(PathBuf::from(&work))
            .output()
            .unwrap();
        assert_eq!(diff, String::from_utf8_lossy(&expected.stdout).trim());
    }

    #[tokio::test]
    async fn flush_makes_a_new_commit_visible() {
        let (_fx, work) = Fixture::new("flush");
        let dir = PathBuf::from(&work);
        let before = diff_base(&work, "main", "vs-main", None).await.unwrap();

        git(&dir, &["switch", "-c", "feature"]);
        git(&dir, &["push", "origin", "main"]); // fast-forward origin/main to HEAD
        git(&dir, &["fetch", "origin"]);
        super::cache::flush();

        let after = diff_base(&work, "feature", "vs-main", None).await.unwrap();
        assert_ne!(
            before, after,
            "the fetched origin/main moved the merge base"
        );
    }

    #[tokio::test]
    async fn default_branch_reads_the_symref_not_the_shorthand() {
        let (_fx, work) = Fixture::new("default");
        assert_eq!(default_branch(&work).await.as_deref(), Some("main"));
    }

    #[tokio::test]
    async fn the_set_head_repair_stamps_its_throttle() {
        let (_fx, work) = Fixture::new("sethead");
        let dir = PathBuf::from(&work);
        git(&dir, &["remote", "set-head", "origin", "--delete"]);
        cache::flush();
        assert_eq!(default_branch(&work).await.as_deref(), Some("main"));
        assert!(
            !cache::shared().due(&format!("set-head:{work}"), SET_HEAD_THROTTLE),
            "a second miss inside the window must not reach the remote"
        );
    }
}
