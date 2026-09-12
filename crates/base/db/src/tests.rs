use super::Db;

async fn tables(db: &Db) -> Vec<String> {
    sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .fetch_all(db.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn an_in_memory_database_carries_the_whole_schema() {
    let db = Db::in_memory().await.unwrap();
    let tables = tables(&db).await;
    for expected in [
        "sessions",
        "repos",
        "worktrees",
        "annotations",
        "pending_confirmations",
    ] {
        assert!(
            tables.iter().any(|t| t == expected),
            "missing {expected} in {tables:?}"
        );
    }
}

#[tokio::test]
async fn opening_a_path_creates_the_file_in_wal_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.db");
    let db = Db::open(&path).await.unwrap();
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(mode, "wal");
    assert!(path.exists());
}

#[tokio::test]
async fn opening_twice_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.db");
    let first = Db::open(&path).await.unwrap();
    drop(first);
    let second = Db::open(&path).await.unwrap();
    assert!(!tables(&second).await.is_empty());
}

#[test]
fn migrations_are_numbered_without_gaps() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut numbers: Vec<u32> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name
                .strip_suffix(".sql")
                .unwrap_or_else(|| panic!("not a migration: {name}"));
            assert!(
                stem.get(4..5) == Some("_"),
                "a migration is named 00NN_what_it_does.sql: {name}"
            );
            stem[..4].parse().unwrap()
        })
        .collect();
    numbers.sort_unstable();
    let expected: Vec<u32> = (1..=numbers.len() as u32).collect();
    assert_eq!(numbers, expected);
}
