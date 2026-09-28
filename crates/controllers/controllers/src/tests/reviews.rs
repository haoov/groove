//! Reviewing someone else's MR: the session it opens and the branch it checks out.

use groove_types::{Forge, ReviewMr, SessionId, SessionKind, Timestamp};

use crate::tests::fixture::{pooled_clone, sh, until};
use crate::{Command as Cmd, SyncSpawner, dispatch, session};

/// An MR on the fixture's own repo, its source branch already on origin.
fn asked() -> ReviewMr {
    ReviewMr {
        forge: Forge::Gitlab,
        project: "g/mayo".into(),
        iid: 7,
        title: "fix: the parser".into(),
        author: "someone".into(),
        source_branch: "release/1.0".into(),
        target_branch: "main".into(),
        draft: false,
        web_url: "https://gitlab.example.com/g/mayo/-/merge_requests/7".into(),
        updated_at: Timestamp::new(0),
        local_path: None,
        approved: false,
        review: None,
    }
}

fn opened(state: &mut crate::AppState, services: &crate::Services, spawner: &SyncSpawner) {
    dispatch(
        Cmd::Session(session::Command::OpenReview {
            project: "g/mayo".into(),
            iid: 7,
        }),
        state,
        services,
        spawner,
    );
}

#[test]
fn a_review_opens_a_session_of_its_own_on_the_mrs_branch() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    dispatch(
        Cmd::Session(session::Command::ListRepos),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.session.pool.is_empty()
    });
    state.delivery.reviews = vec![asked()];

    opened(&mut state, &services, &spawner);
    let id = SessionId::new("review-g-mayo-7");
    until(&spawner, &services, &mut state, |s| {
        s.session
            .get(&id)
            .is_some_and(|open| !open.worktrees.is_empty())
            || !s.errors.is_empty()
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);

    let open = state.session.get(&id).expect("the review's session");
    assert_eq!(
        open.session.kind,
        SessionKind::Review {
            project: "g/mayo".into(),
            iid: 7
        }
    );
    assert_eq!(open.session.title, "fix: the parser");
    let worktree = open.worktrees.first().expect("its worktree");
    assert_eq!(worktree.branch, "release/1.0", "the mr's own branch");
    assert_eq!(worktree.base_ref.as_deref(), Some("main"));
    let head = sh(
        std::path::Path::new(&worktree.path),
        &["branch", "--show-current"],
    );
    assert_eq!(head.trim(), "release/1.0", "checked out where the mr is");
    assert!(
        state.agent.agent(&id).is_some(),
        "its agent is started with it"
    );
}

#[test]
fn opening_the_same_review_again_selects_the_session_it_already_has() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    dispatch(
        Cmd::Session(session::Command::ListRepos),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.session.pool.is_empty()
    });
    state.delivery.reviews = vec![asked()];
    opened(&mut state, &services, &spawner);
    let id = SessionId::new("review-g-mayo-7");
    until(&spawner, &services, &mut state, |s| {
        s.session
            .get(&id)
            .is_some_and(|open| !open.worktrees.is_empty())
    });
    let held = state.session.open.len();

    state.session.selected = None;
    opened(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.session.selected.is_some()
    });
    assert_eq!(state.session.open.len(), held, "no second session for it");
    assert_eq!(state.session.selected.as_ref(), Some(&id));
}

#[test]
fn an_mr_the_queue_does_not_hold_opens_nothing() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    opened(&mut state, &services, &spawner);
    assert!(state.session.open.is_empty());
    assert!(state.errors.is_empty(), "and nothing is reported");
}

#[test]
fn a_review_takes_the_clone_the_pool_holds_without_listing_it_first() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    state.delivery.reviews = vec![asked()];
    assert!(state.session.pool.is_empty(), "nothing has listed it yet");

    opened(&mut state, &services, &spawner);
    let id = SessionId::new("review-g-mayo-7");
    until(&spawner, &services, &mut state, |s| {
        s.session
            .get(&id)
            .is_some_and(|open| !open.worktrees.is_empty())
            || !s.errors.is_empty()
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let open = state.session.get(&id).expect("the review's session");
    assert_eq!(open.repos.len(), 1, "the pool's own clone, not a new one");
}

#[test]
fn a_review_opens_on_the_whole_change_and_a_task_on_what_is_uncommitted() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    state.delivery.reviews = vec![asked()];

    opened(&mut state, &services, &spawner);
    let id = SessionId::new("review-g-mayo-7");
    until(&spawner, &services, &mut state, |s| {
        s.session
            .get(&id)
            .is_some_and(|open| !open.worktrees.is_empty())
    });
    assert_eq!(
        state.workspace.mode,
        groove_types::DiffMode::Base,
        "a review reads the branch's whole change"
    );

    let explorer = crate::tests::fixture::worktree(&mut state, &services, &spawner);
    assert!(!explorer.is_empty());
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert_eq!(
        state.workspace.mode,
        groove_types::DiffMode::Working,
        "every other session reads what is not committed"
    );
}
