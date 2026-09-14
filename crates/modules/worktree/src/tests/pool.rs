use crate::tests::fixture::{Fixture, SLUG, sh};
use crate::{Clone, Error, Pool};

#[tokio::test]
async fn the_pool_lists_registers_and_resolves() {
    let fx = Fixture::new().await;
    let clones = fx.pool.list();
    assert_eq!(clones.len(), 1);
    assert_eq!(clones[0].slug, SLUG);
    assert_eq!(clones[0].path, fx.clone);

    assert_eq!(Pool::resolve("mayo", &clones).unwrap().slug, SLUG);
    assert_eq!(Pool::resolve("devops/mayo", &clones).unwrap().slug, SLUG);
    assert_eq!(Pool::resolve(SLUG, &clones).unwrap().slug, SLUG);
    assert!(
        matches!(Pool::resolve("ayo", &clones), Err(Error::UnknownRepo(_))),
        "no partial words"
    );

    let twin = Clone {
        slug: "github.com/other/mayo".into(),
        path: fx.root.path().join("x"),
    };
    let both = vec![clones[0].clone(), twin];
    assert!(matches!(
        Pool::resolve("mayo", &both),
        Err(Error::AmbiguousRepo { .. })
    ));

    let repo = fx.repo().await;
    assert_eq!(repo.id.as_str(), SLUG);
    assert_eq!(
        (
            repo.host.as_str(),
            repo.group_path.as_str(),
            repo.project.as_str()
        ),
        ("gitlab.example.com", "wiremind/devops", "mayo")
    );
    assert_eq!(fx.pool.repo(&repo.id).await.unwrap(), repo);
    assert_eq!(
        Pool::new(fx.pool.db.clone(), fx.root.path().join("nowhere")).list(),
        Vec::<Clone>::new()
    );
}

#[tokio::test]
async fn a_clone_without_origin_is_refused() {
    let fx = Fixture::new().await;
    sh(&fx.clone, &["remote", "remove", "origin"]);
    let clones = fx.pool.list();
    assert!(matches!(
        fx.pool.register(&clones[0]).await,
        Err(Error::NoOrigin { .. })
    ));
}

#[tokio::test]
async fn clone_puts_the_repo_in_its_place_and_refuses_a_second_time() {
    let fx = Fixture::new().await;
    let dest = fx.root.path().join("main/github.com/owner/proj");
    let repo = fx
        .pool
        .clone(&format!("file://{}", fx.origin.display()))
        .await;
    assert!(
        matches!(repo, Err(Error::Git(_))),
        "a file url has no host and group"
    );

    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    sh(
        dest.parent().unwrap(),
        &["clone", fx.origin.to_str().unwrap(), "proj"],
    );
    sh(
        &dest,
        &[
            "remote",
            "set-url",
            "origin",
            "git@github.com:owner/proj.git",
        ],
    );
    let err = fx
        .pool
        .clone("git@github.com:owner/proj.git")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Exists { .. }), "{err}");
    assert_eq!(fx.pool.list().len(), 2);
}
