use super::super::models::{NotionTask, Repo, SessionKind};
use super::super::test_pool;
use super::*;

fn repo(id: &str) -> Repo {
    Repo {
        id: id.to_string(),
        host: "gitlab.example.com".into(),
        group_path: "group".into(),
        project: id.rsplit('/').next().unwrap().to_string(),
        local_path: format!("/pool/{id}"),
    }
}

fn mirror_task(short_id: &str) -> NotionTask {
    NotionTask {
        page_id: format!("page-{short_id}"),
        short_id: short_id.to_string(),
        title: format!("Title of {short_id}"),
        status: "Ready".into(),
        priority: Some("High".into()),
        synced_at: 1,
    }
}

#[tokio::test]
async fn removing_a_session_cascades_to_everything_it_owns() {
    let pool = test_pool().await;
    repos::upsert(&pool, &repo("g/a")).await.unwrap();
    sessions::create_explorer(&pool, "explorer-1", "X").await.unwrap();
    repos::attach(&pool, "explorer-1", "g/a").await.unwrap();
    let wt = worktrees::upsert(&pool, "explorer-1", "g/a", "explorer/x", "/wt/explorer-1/a@explorer-x")
        .await
        .unwrap();
    mrs::upsert(&pool, &wt.id, "gitlab", "42", "https://x/42", "open").await.unwrap();
    annotations::create(&pool, "explorer-1", "g/a", "f.rs", 1, 3, "issue: x", "human")
        .await
        .unwrap();
    time::add(&pool, "explorer-1", "2026-08-19", 60).await.unwrap();

    sessions::remove(&pool, "explorer-1").await.unwrap();

    assert!(worktrees::for_session(&pool, "explorer-1").await.unwrap().is_empty());
    assert!(mrs::for_worktree(&pool, &wt.id).await.unwrap().is_empty());
    assert!(annotations::for_session(&pool, "explorer-1", None).await.unwrap().is_empty());
    assert!(repos::attached_to(&pool, "explorer-1").await.unwrap().is_empty());
    let t = time::summary(&pool, "explorer-1", "2026-08-19").await.unwrap();
    assert_eq!(t.tracked_seconds, 0);
}

#[tokio::test]
async fn adopting_an_explorer_carries_children_to_the_new_id() {
    let pool = test_pool().await;
    repos::upsert(&pool, &repo("g/a")).await.unwrap();
    sessions::create_explorer(&pool, "explorer-2", "Scratch").await.unwrap();
    repos::attach(&pool, "explorer-2", "g/a").await.unwrap();
    let wt = worktrees::upsert(&pool, "explorer-2", "g/a", "explorer/scratch", "/wt/explorer-2/a@explorer-scratch")
        .await
        .unwrap();
    annotations::create(&pool, "explorer-2", "g/a", "f.rs", 1, 1, "note: x", "agent")
        .await
        .unwrap();

    let task = mirror_task("TASKS2-9");
    sessions::adopt_explorer(
        &pool,
        "explorer-2",
        &task,
        &[(wt.id.clone(), "/wt/TASKS2-9/a".to_string())],
        "tasks2-9",
    )
    .await
    .unwrap();

    let adopted = sessions::get(&pool, "TASKS2-9").await.unwrap();
    assert_eq!(adopted.kind, SessionKind::Task);
    assert_eq!(adopted.notion_page_id.as_deref(), Some("page-TASKS2-9"));
    assert!(sessions::get_opt(&pool, "explorer-2").await.unwrap().is_none());

    let moved = worktrees::for_session(&pool, "TASKS2-9").await.unwrap();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].branch, "tasks2-9");
    assert_eq!(moved[0].path, "/wt/TASKS2-9/a");
    assert_eq!(annotations::for_session(&pool, "TASKS2-9", None).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_repo_detaches_only_with_its_last_worktree() {
    let pool = test_pool().await;
    repos::upsert(&pool, &repo("g/a")).await.unwrap();
    sessions::create_explorer(&pool, "explorer-3", "X").await.unwrap();
    repos::attach(&pool, "explorer-3", "g/a").await.unwrap();
    let first = worktrees::upsert(&pool, "explorer-3", "g/a", "b1", "/wt/explorer-3/a@b1")
        .await
        .unwrap();
    let second = worktrees::upsert(&pool, "explorer-3", "g/a", "b2", "/wt/explorer-3/a@b2")
        .await
        .unwrap();
    assert_ne!(first.id, second.id, "two branches, two worktrees, one repo");

    worktrees::close(&pool, &first.id).await.unwrap();
    assert_eq!(
        repos::attached_to(&pool, "explorer-3").await.unwrap().len(),
        1,
        "one worktree left keeps the repo attached"
    );

    worktrees::close(&pool, &second.id).await.unwrap();
    assert!(repos::attached_to(&pool, "explorer-3").await.unwrap().is_empty());
}

#[tokio::test]
async fn upserting_the_same_branch_reuses_the_worktree_row() {
    let pool = test_pool().await;
    repos::upsert(&pool, &repo("g/a")).await.unwrap();
    sessions::create_explorer(&pool, "explorer-6", "X").await.unwrap();
    let first = worktrees::upsert(&pool, "explorer-6", "g/a", "b1", "/wt/explorer-6/a@b1")
        .await
        .unwrap();
    let again = worktrees::upsert(&pool, "explorer-6", "g/a", "b1", "/wt/moved/a@b1")
        .await
        .unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(again.path, "/wt/moved/a@b1");
    assert_eq!(worktrees::for_repo(&pool, "explorer-6", "g/a").await.unwrap().len(), 1);
}

#[tokio::test]
async fn review_identity_is_project_and_iid() {
    let pool = test_pool().await;
    let first = sessions::upsert_review(&pool, "review-g-a-7", "g/a", 7, "Fix x")
        .await
        .unwrap();
    let again = sessions::upsert_review(&pool, "review-other-id", "g/a", 7, "Fix x v2")
        .await
        .unwrap();
    assert_eq!(first.id, again.id, "same MR resumes the same session");
    assert_eq!(again.title, "Fix x v2");
    assert_eq!(again.kind, SessionKind::Review);
}

#[tokio::test]
async fn time_ledger_sums_and_never_goes_negative() {
    let pool = test_pool().await;
    sessions::create_explorer(&pool, "explorer-4", "X").await.unwrap();
    time::add(&pool, "explorer-4", "2026-08-18", 600).await.unwrap();
    time::add(&pool, "explorer-4", "2026-08-19", 300).await.unwrap();
    time::add(&pool, "explorer-4", "2026-08-19", 300).await.unwrap();

    let t = time::summary(&pool, "explorer-4", "2026-08-19").await.unwrap();
    assert_eq!(t.tracked_seconds, 1200);
    assert_eq!(t.today_seconds, 600);
    assert_eq!(t.unlogged_seconds, 1200);

    time::log(&pool, "explorer-4", 3600).await.unwrap();
    let t = time::summary(&pool, "explorer-4", "2026-08-19").await.unwrap();
    assert_eq!(t.logged_seconds, 3600);
    assert_eq!(t.unlogged_seconds, 0, "over-logging never yields a negative remainder");
}

#[tokio::test]
async fn open_task_requires_the_mirror_and_is_idempotent() {
    let pool = test_pool().await;
    assert!(sessions::open_task(&pool, "TASKS2-1").await.is_err());

    notion_tasks::upsert(&pool, &mirror_task("TASKS2-1")).await.unwrap();
    let session = sessions::open_task(&pool, "TASKS2-1").await.unwrap();
    let reopened = sessions::open_task(&pool, "TASKS2-1").await.unwrap();
    assert_eq!(session.id, reopened.id);

    let view = sessions::view(&pool, "TASKS2-1").await.unwrap();
    assert_eq!(view.status, "Ready", "status comes from the mirror");
}

#[tokio::test]
async fn mr_upsert_is_idempotent_per_worktree_and_number() {
    let pool = test_pool().await;
    repos::upsert(&pool, &repo("g/a")).await.unwrap();
    sessions::create_explorer(&pool, "explorer-5", "X").await.unwrap();
    let wt = worktrees::upsert(&pool, "explorer-5", "g/a", "b", "/wt/explorer-5/a")
        .await
        .unwrap();

    let first = mrs::upsert(&pool, &wt.id, "gitlab", "42", "https://x/42", "open").await.unwrap();
    let second = mrs::upsert(&pool, &wt.id, "gitlab", "42", "https://x/42", "merged").await.unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.state, "merged");
    assert_eq!(mrs::for_worktree(&pool, &wt.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn confirmations_claim_once_and_order_by_age() {
    let pool = test_pool().await;
    confirmations::insert(&pool, "c1", None, "git.push", "{}", "ui").await.unwrap();
    confirmations::insert(&pool, "c2", None, "git.pull", "{}", "mcp").await.unwrap();

    assert!(confirmations::identical_pending(&pool, "git.push", None, "{}").await.unwrap());
    let all = confirmations::all(&pool).await.unwrap();
    assert_eq!(all.len(), 2);

    let claimed = confirmations::claim(&pool, "c1").await.unwrap();
    assert_eq!(claimed.unwrap().op_type, "git.push");
    assert!(confirmations::claim(&pool, "c1").await.unwrap().is_none(), "second claim loses");
}

/// The store is the only module that speaks SQL. A raw query anywhere else is
/// a review failure made mechanical.
#[test]
fn no_raw_sql_outside_the_store() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = vec![];
    visit(&src, &mut offenders);
    assert!(offenders.is_empty(), "sqlx::query outside core/db: {offenders:?}");

    fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, offenders);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.to_string_lossy().contains("/core/db/")
                && std::fs::read_to_string(&path).unwrap().contains("sqlx::query")
            {
                offenders.push(path.display().to_string());
            }
        }
    }
}
