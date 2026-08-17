//! One-shot move from the first on-disk layout to the current one.
//!
//! Was `<root>/MAIN/<group>/<project>` for the clones, with a task directory per
//! task beside it at the root. Now the clones sit under `<root>/main/<host>/…` and
//! the tasks under `<root>/worktrees/`.
//!
//! Moving a worktree or its clone breaks the absolute paths the two hold on each
//! other, so every move is followed by `git worktree repair`, and the recorded
//! paths are rewritten in the same pass. Nothing here needs a marker file: with no
//! `MAIN` and no task directory at the root there is nothing to find, so a second
//! run is a no-op — and a run interrupted half way is simply finished by the next.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

/// How deep to look for clones under the old `MAIN` (group paths nest).
const MAX_DEPTH: u32 = 5;

/// A directory that moved, old path to new.
struct Move {
    from: PathBuf,
    to: PathBuf,
}

/// `root` is the configured worktree root — passed in rather than read here, so a
/// test can hand this a synthetic one.
pub async fn run(root: &Path, pool: &SqlitePool) {
    if !root.is_dir() {
        return;
    }
    let legacy = root.join("MAIN");
    let pool_dir = root.join("main");

    // A case-insensitive filesystem would report the new pool as the old one, and
    // "migrating" it onto itself would delete it.
    if same_dir(&legacy, &pool_dir) {
        return;
    }

    let clones = if legacy.is_dir() { move_clones(&legacy, pool, root).await } else { vec![] };
    let tasks = move_task_dirs(root, pool).await;

    if clones.is_empty() && tasks.is_empty() {
        return;
    }

    // Repair after everything has moved, so both ends of each link are final.
    let repaired = repair_worktrees(root, &clones).await;

    // The old MAIN holds nothing but the group directories its clones sat in.
    prune_empty(&legacy);

    tracing::info!(
        "[layout] moved {} clones and {} task directories, repaired {repaired} worktrees",
        clones.len(),
        tasks.len(),
    );
    crate::events::notice(
        "info",
        "git",
        "Worktree root reorganised".to_string(),
        Some(format!(
            "{} clones are now under main/<host>/, and {} task directories under worktrees/.",
            clones.len(),
            tasks.len(),
        )),
        None,
    );
}

/// True when both paths exist and name the same directory.
fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Move every clone out of the old `MAIN`, under its host.
async fn move_clones(legacy: &Path, pool: &SqlitePool, root: &Path) -> Vec<Move> {
    let mut moved = vec![];
    for clone in find_clones(legacy) {
        let Ok(rel) = clone.strip_prefix(legacy) else { continue };
        let Some(host) = host_of(&clone, pool).await else {
            tracing::warn!("[layout] no host for {} — left where it is", clone.display());
            continue;
        };
        let dest = root.join("main").join(&host).join(rel);
        if !rename(&clone, &dest) {
            continue;
        }
        // The pool path is recorded per repo; a stale one reads as a missing repo.
        let _ = sqlx::query("UPDATE repos SET local_path = ? WHERE local_path = ?")
            .bind(dest.to_string_lossy().to_string())
            .bind(clone.to_string_lossy().to_string())
            .execute(pool)
            .await;
        moved.push(Move { from: clone, to: dest });
    }
    moved
}

/// Directories holding a `.git` — a clone. Never descends into one.
fn find_clones(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: u32, acc: &mut Vec<PathBuf>) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.join(".git").exists() {
                acc.push(path);
            } else {
                walk(&path, depth + 1, acc);
            }
        }
    }
    let mut acc = vec![];
    walk(dir, 1, &mut acc);
    acc
}

/// The forge host for a clone: what the repo row says, else what its remote says.
async fn host_of(clone: &Path, pool: &SqlitePool) -> Option<String> {
    let path = clone.to_string_lossy().to_string();
    let recorded: Option<(String,)> = sqlx::query_as("SELECT host FROM repos WHERE local_path = ?")
        .bind(&path)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    if let Some((host,)) = recorded {
        return Some(host);
    }
    let url = super::run_git(&path, &["remote", "get-url", "origin"]).await.ok()?;
    super::parse_git_url(url.trim()).ok().map(|(host, _, _)| host)
}

/// Move each task directory at the root into `worktrees/`.
async fn move_task_dirs(root: &Path, pool: &SqlitePool) -> Vec<Move> {
    let dest_root = root.join("worktrees");
    let mut moved = vec![];
    let Ok(entries) = std::fs::read_dir(root) else { return moved };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || name.starts_with('.') || matches!(name.as_str(), "MAIN" | "main" | "worktrees") {
            continue;
        }
        // A worktree inside says "task directory". Failing that, a directory holding
        // nothing but hidden files is one we emptied ourselves — a closed task can
        // leave `.agent_session_id` behind — and comes along. Anything with real
        // content in it is the user's, and stays where they put it.
        if !holds_worktree(&path) && !holds_only_hidden(&path) {
            tracing::info!("[layout] {} is not a task directory — left alone", path.display());
            continue;
        }
        let dest = dest_root.join(&name);
        if !rename(&path, &dest) {
            continue;
        }
        // Every worktree of this task at once — the tail of the path is unchanged.
        let old_prefix = format!("{}/", path.to_string_lossy());
        let new_prefix = format!("{}/", dest.to_string_lossy());
        let _ = sqlx::query("UPDATE worktrees SET path = ? || substr(path, ?) WHERE path LIKE ? || '%'")
            .bind(&new_prefix)
            .bind(old_prefix.len() as i64 + 1)
            .bind(&old_prefix)
            .execute(pool)
            .await;
        moved.push(Move { from: path, to: dest });
    }
    moved
}

/// True when a child of `dir` is a linked worktree (its `.git` is a FILE pointing
/// into a clone, where a clone's own `.git` is a directory).
fn holds_worktree(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries.flatten().any(|e| e.path().join(".git").is_file())
}

/// Delete `dir` and everything under it that is empty, bottom-up. A directory with
/// real content in it survives, and so does every parent of that content.
fn prune_empty(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            prune_empty(&path);
        }
    }
    let _ = std::fs::remove_dir(dir);
}

/// True for an empty directory, and for one holding only dot-files.
fn holds_only_hidden(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries
        .flatten()
        .all(|e| e.file_name().to_string_lossy().starts_with('.'))
}

fn rename(from: &Path, to: &Path) -> bool {
    if to.exists() {
        tracing::warn!("[layout] {} already exists — {} left alone", to.display(), from.display());
        return false;
    }
    if let Some(parent) = to.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("[layout] cannot create {}: {e}", parent.display());
            return false;
        }
    }
    match std::fs::rename(from, to) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!("[layout] cannot move {} to {}: {e}", from.display(), to.display());
            false
        }
    }
}

/// Point every moved worktree and its clone back at each other.
///
/// `git worktree repair <path>`, run from the clone, is the one command that fixes
/// both ends: the worktree's `.git` file and the clone's own record of where that
/// worktree lives. Which clone to run it from is read out of the worktree itself,
/// through the moves, because its `.git` file still names the old location.
async fn repair_worktrees(root: &Path, clones: &[Move]) -> usize {
    let mut repaired = 0;
    // First from each moved clone with no path: that fixes the worktrees whose OWN
    // path did not change — one living outside this root, say. The ones that moved
    // are still recorded at their old path, so git skips them here, and the pass
    // below names each of those explicitly.
    for m in clones {
        let _ = super::run_git_output(&m.to.to_string_lossy(), &["worktree", "repair"]).await;
    }
    let Ok(tasks) = std::fs::read_dir(root.join("worktrees")) else { return 0 };
    for task in tasks.flatten() {
        let Ok(repos) = std::fs::read_dir(task.path()) else { continue };
        for repo in repos.flatten() {
            let wt = repo.path();
            let dot_git = wt.join(".git");
            if !dot_git.is_file() {
                continue;
            }
            let Some(clone) = clone_of(&dot_git, clones) else {
                tracing::warn!("[layout] cannot tell which clone {} belongs to", wt.display());
                continue;
            };
            let wt_str = wt.to_string_lossy().to_string();
            match super::run_git_output(&clone.to_string_lossy(), &["worktree", "repair", &wt_str]).await {
                Ok(out) if out.status.success() => repaired += 1,
                Ok(out) => tracing::warn!(
                    "[layout] repair of {wt_str} failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
                Err(e) => tracing::warn!("[layout] repair of {wt_str} failed: {e}"),
            }
        }
    }
    repaired
}

/// The clone a worktree belongs to, at its new location. Its `.git` file reads
/// `gitdir: <clone>/.git/worktrees/<name>`, which still names where the clone was.
fn clone_of(dot_git: &Path, clones: &[Move]) -> Option<PathBuf> {
    let text = std::fs::read_to_string(dot_git).ok()?;
    let recorded = text.trim().strip_prefix("gitdir:")?.trim();
    let admin = Path::new(recorded);
    // Strip the `/.git/worktrees/<name>` tail to get the clone itself.
    let clone = admin.ancestors().nth(3)?;
    // A clone that moved answers under its new path; one that never moved (a repo
    // registered from outside the pool) is already correct.
    clones
        .iter()
        .find(|m| m.from == clone)
        .map(|m| m.to.clone())
        .or_else(|| clone.is_dir().then(|| clone.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=T", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }

    fn git_out(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git").args(args).current_dir(dir).output().expect("git runs");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A real database file, not `:memory:` — a pool hands out several connections
    /// and each in-memory one would be its own empty database.
    async fn pool_with(
        dir: &Path,
        repos: &[(&str, &str, &str)],
        worktrees: &[(&str, &str, &str)],
    ) -> SqlitePool {
        // foreign_keys(false) mirrors db::init: the app re-points child rows in an
        // order enforcement would reject, so the test schema must behave the same.
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(dir.join("test.db"))
                .create_if_missing(true)
                .foreign_keys(false),
        )
        .await
        .unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        for (id, host, path) in repos {
            sqlx::query("INSERT INTO repos (id, host, group_path, project, local_path) VALUES (?, ?, 'g', 'p', ?)")
                .bind(id).bind(host).bind(path)
                .execute(&pool).await.unwrap();
        }
        for (id, task, path) in worktrees {
            sqlx::query(
                "INSERT INTO worktrees (id, task_id, repo_id, branch, path, is_active, created_at)
                 VALUES (?, ?, 'r', 'b', ?, 1, 0)",
            )
            .bind(id).bind(task).bind(path)
            .execute(&pool).await.unwrap();
        }
        pool
    }

    struct Root(PathBuf);
    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A root in the old shape: one clone under MAIN with a nested group, a task
    /// worktree of it, an empty leftover, and a directory that is the user's own.
    fn old_root(name: &str) -> (Root, PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("groove-layout-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let guard = Root(root.clone());

        let clone = root.join("MAIN/wiremind/devops/mayo");
        std::fs::create_dir_all(&clone).unwrap();
        git(&clone, &["init", "--initial-branch=main", "."]);
        git(&clone, &["remote", "add", "origin", "git@gitlab.example.com:wiremind/devops/mayo.git"]);
        std::fs::write(clone.join("a.txt"), "one\n").unwrap();
        git(&clone, &["add", "."]);
        git(&clone, &["commit", "-m", "first"]);

        let wt = root.join("TASKS2-1/mayo");
        git(&clone, &["worktree", "add", "-b", "tasks2-1", wt.to_str().unwrap()]);

        std::fs::create_dir_all(root.join("explorer-9")).unwrap();       // empty leftover
        let closed = root.join("TASKS2-2");                              // ours, emptied
        std::fs::create_dir_all(&closed).unwrap();
        std::fs::write(closed.join(".agent_session_id"), "uuid\n").unwrap();
        std::fs::create_dir_all(root.join("notes/deep")).unwrap();       // the user's own
        std::fs::write(root.join("notes/deep/keep.md"), "mine\n").unwrap();
        std::fs::write(root.join("CLAUDE.md"), "guide\n").unwrap();

        (guard, root, clone, wt)
    }

    #[tokio::test]
    async fn moves_clones_under_their_host_and_tasks_under_worktrees() {
        let (_g, root, clone, wt) = old_root("move");
        let pool = pool_with(
            &root,
            &[("mayo", "gitlab.example.com", clone.to_str().unwrap())],
            &[("w1", "TASKS2-1", wt.to_str().unwrap())],
        )
        .await;

        run(&root, &pool).await;

        let new_clone = root.join("main/gitlab.example.com/wiremind/devops/mayo");
        let new_wt = root.join("worktrees/TASKS2-1/mayo");
        assert!(new_clone.join(".git").is_dir(), "clone moved under its host");
        assert!(new_wt.join(".git").is_file(), "worktree moved under worktrees/");
        assert!(!root.join("MAIN").exists(), "the old pool is gone");
        assert!(root.join("worktrees/explorer-9").is_dir(), "an empty leftover comes along");
        assert!(
            root.join("worktrees/TASKS2-2/.agent_session_id").is_file(),
            "a closed task holding only our own marker comes along too",
        );
        // Not ours: a directory with real content stays where the user put it.
        assert!(root.join("notes/deep/keep.md").is_file());
        assert!(root.join("CLAUDE.md").is_file());

        // The recorded paths follow the move, or the app looks at empty directories.
        let repo_path: (String,) = sqlx::query_as("SELECT local_path FROM repos WHERE id = 'mayo'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(repo_path.0, new_clone.to_string_lossy());
        let wt_path: (String,) = sqlx::query_as("SELECT path FROM worktrees WHERE id = 'w1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(wt_path.0, new_wt.to_string_lossy());
    }

    #[tokio::test]
    async fn the_moved_worktree_and_its_clone_still_know_each_other() {
        let (_g, root, clone, wt) = old_root("repair");
        let pool = pool_with(
            &root,
            &[("mayo", "gitlab.example.com", clone.to_str().unwrap())],
            &[("w1", "TASKS2-1", wt.to_str().unwrap())],
        )
        .await;

        run(&root, &pool).await;

        let new_clone = root.join("main/gitlab.example.com/wiremind/devops/mayo");
        let new_wt = root.join("worktrees/TASKS2-1/mayo");
        // Without `git worktree repair` both of these fail: the worktree's .git file
        // and the clone's record of it still hold the paths from before the move.
        assert_eq!(git_out(&new_wt, &["rev-parse", "--abbrev-ref", "HEAD"]), "tasks2-1");
        assert!(
            git_out(&new_clone, &["worktree", "list"]).contains(new_wt.to_str().unwrap()),
            "the clone lists the worktree at its new path"
        );
        assert_eq!(git_out(&new_wt, &["status", "--porcelain"]), "");
    }

    #[tokio::test]
    async fn a_root_already_in_the_new_shape_is_left_alone() {
        let (_g, root, clone, wt) = old_root("idempotent");
        let pool = pool_with(
            &root,
            &[("mayo", "gitlab.example.com", clone.to_str().unwrap())],
            &[("w1", "TASKS2-1", wt.to_str().unwrap())],
        )
        .await;

        run(&root, &pool).await;
        let before = std::fs::read_dir(root.join("main/gitlab.example.com/wiremind/devops")).unwrap().count();
        run(&root, &pool).await; // a second launch

        assert_eq!(
            std::fs::read_dir(root.join("main/gitlab.example.com/wiremind/devops")).unwrap().count(),
            before,
        );
        assert!(root.join("worktrees/TASKS2-1/mayo/.git").is_file());
        assert!(!root.join("worktrees/main").exists(), "the pool is not a task directory");
        assert!(!root.join("worktrees/worktrees").exists());
    }
}
