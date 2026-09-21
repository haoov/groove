use groove_types::{Forge, MrDetails, MrState, Timestamp, WorktreeId};

use crate::{Snapshot, Store};

fn worktree() -> WorktreeId {
    WorktreeId::new("w1")
}

/// The rows an MR hangs off: a session, a repo, a worktree.
async fn store() -> Store {
    let store = Store::in_memory().await.expect("a database");
    for statement in [
        "INSERT INTO sessions (id, kind, title, created_at)
         VALUES ('s1', 'explorer', 'work', 0)",
        "INSERT INTO repos (id, host, group_path, project, local_path)
         VALUES ('r1', 'github.com', 'acme', 'groove', '/main/groove')",
        "INSERT INTO worktrees (id, session_id, repo_id, branch, path, created_at)
         VALUES ('w1', 's1', 'r1', 'fix/one', '/w/one', 0)",
    ] {
        sqlx::query(statement)
            .execute(store.db().pool())
            .await
            .expect("the row is seeded");
    }
    store
}

fn snapshot(number: &str, state: MrState, url: &str) -> Snapshot {
    Snapshot {
        node: "PR_node".into(),
        number: number.to_string(),
        details: MrDetails {
            title: "fix: one".into(),
            description: String::new(),
            author: "haoov".into(),
            source_branch: "fix/one".into(),
            target_branch: "main".into(),
            state,
            draft: false,
            created_at: Timestamp::new(0),
            updated_at: Timestamp::new(0),
            web_url: url.to_string(),
            approval: None,
            reviewers: Vec::new(),
        },
        ci: None,
        threads: Vec::new(),
    }
}

#[tokio::test]
async fn a_worktree_with_no_mr_holds_none() {
    let store = store().await;
    assert!(store.get(&worktree()).await.unwrap().is_none());
}

#[tokio::test]
async fn what_the_forge_answered_reads_back_as_the_worktrees_mr() {
    let store = store().await;
    let read = snapshot("7", MrState::Open, "https://github.com/acme/groove/pull/7");
    let saved = store
        .save(&worktree(), Forge::Github, &read)
        .await
        .expect("the mr is written");
    assert_eq!(saved.remote_id, "7");
    assert_eq!(saved.forge, Forge::Github);
    assert_eq!(saved.state, MrState::Open);
    assert_eq!(saved.worktree, worktree());
    assert_eq!(store.get(&worktree()).await.unwrap(), Some(saved));
}

#[tokio::test]
async fn the_same_mr_saved_again_keeps_its_id_and_takes_the_new_state() {
    let store = store().await;
    let open = snapshot("7", MrState::Open, "https://github.com/acme/groove/pull/7");
    let first = store.save(&worktree(), Forge::Github, &open).await.unwrap();
    let merged = snapshot(
        "7",
        MrState::Merged,
        "https://github.com/acme/groove/pull/7",
    );
    let again = store
        .save(&worktree(), Forge::Github, &merged)
        .await
        .unwrap();
    assert_eq!(again.id, first.id, "one row, updated");
    assert_eq!(again.state, MrState::Merged);
}

#[tokio::test]
async fn a_second_mr_on_the_branch_replaces_the_first() {
    let store = store().await;
    let first = snapshot(
        "7",
        MrState::Closed,
        "https://github.com/acme/groove/pull/7",
    );
    store
        .save(&worktree(), Forge::Github, &first)
        .await
        .unwrap();
    let second = snapshot("8", MrState::Open, "https://github.com/acme/groove/pull/8");
    let now = store
        .save(&worktree(), Forge::Github, &second)
        .await
        .unwrap();
    assert_eq!(now.remote_id, "8");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM mrs WHERE worktree_id = 'w1'")
        .fetch_one(store.db().pool())
        .await
        .unwrap();
    assert_eq!(count, 1, "a worktree keeps one mr");
}

#[tokio::test]
async fn the_mr_goes_when_it_is_forgotten_and_when_the_worktree_goes() {
    let store = store().await;
    let read = snapshot("7", MrState::Open, "https://github.com/acme/groove/pull/7");
    store.save(&worktree(), Forge::Github, &read).await.unwrap();
    store.remove(&worktree()).await.unwrap();
    assert!(store.get(&worktree()).await.unwrap().is_none());

    store.save(&worktree(), Forge::Github, &read).await.unwrap();
    sqlx::query("DELETE FROM worktrees WHERE id = 'w1'")
        .execute(store.db().pool())
        .await
        .unwrap();
    assert!(
        store.get(&worktree()).await.unwrap().is_none(),
        "the row cascades with its worktree"
    );
}
