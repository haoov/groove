//! What a gap gives up when the reader opens it.

use super::*;

#[test]
#[allow(clippy::single_range_in_vec_init)]
fn a_gap_gives_up_its_lines_from_the_end_that_was_asked() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let whole: String = (0..60).map(|at| format!("line {at}\n")).collect();
    let path = std::path::Path::new(&dir).join("a.txt");
    std::fs::write(&path, &whole).unwrap();
    let at = std::path::Path::new(&dir);
    sh(at, &["add", "."]);
    sh(at, &["commit", "-m", "the whole file"]);
    std::fs::write(&path, whole.replace("line 0\n", "LINE 0\n")).unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.rows() > 1
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show {
            rows: 0..state.workspace.changes.rows(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("a.txt")
    });

    let gap = gap_row(&state).expect("a gap stands");
    let before = state.workspace.changes.rows();
    dispatch(
        Cmd::Workspace(workspace::Command::OpenGap {
            row: gap,
            way: workspace::Way::Down,
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(
        state.workspace.changes.rows(),
        before + 20,
        "twenty lines stand where the gap was"
    );
    let wanted: Vec<std::ops::Range<u32>> = vec![4..24];
    assert_eq!(
        state.workspace.changes.opened_of("a.txt"),
        wanted.as_slice()
    );

    let gap = gap_row(&state).expect("what is left of it");
    dispatch(
        Cmd::Workspace(workspace::Command::OpenGap {
            row: gap,
            way: workspace::Way::All,
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(gap_row(&state).is_none(), "nothing is hidden any more");
}

/// The first row of the change that hides lines.
fn gap_row(state: &crate::AppState) -> Option<usize> {
    (0..state.workspace.changes.rows()).find(|row| {
        matches!(
            state.workspace.changes.at(*row),
            Some(groove_workspace_service::At::Row(file, at))
                if file.row(at).is_some_and(|one| matches!(one.kind, groove_types::RowKind::Gap(_)))
        )
    })
}

#[test]
fn reading_a_file_keeps_the_colours_the_others_already_have() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    std::fs::write(std::path::Path::new(&dir).join("b.rs"), "fn b() {}\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().len() == 2
    });

    let before = state.workspace.stamp;
    dispatch(
        Cmd::Workspace(workspace::Command::Show {
            rows: 0..state.workspace.changes.rows(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.len() == 2
    });
    assert_eq!(
        state.workspace.stamp, before,
        "a file arriving changes nothing that is already painted"
    );
}

#[test]
fn opening_a_file_the_stream_holds_runs_no_job() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("b.rs"), "fn b() {}\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().iter().any(|f| f.path == "b.rs")
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show {
            rows: 0..state.workspace.changes.rows(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("b.rs")
    });

    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "b.rs".into(),
            at: None,
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state.pending.is_empty(),
        "nothing was read again: {:?}",
        state.pending
    );
    let open = state.workspace.active().expect("the file is open");
    assert_eq!(open.path, "b.rs", "and it is open at once");
}

#[test]
fn a_change_loaded_again_under_the_same_rows_reads_its_files() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let root = std::path::Path::new(&dir);
    std::fs::write(root.join("a.txt"), "two\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .changes
            .files()
            .iter()
            .any(|f| f.path == "a.txt")
    });
    let rows = 0..40;
    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: rows.clone() }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("a.txt")
    });

    std::fs::write(root.join("b.rs"), "fn b() {}\n").unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().iter().any(|f| f.path == "b.rs")
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("b.rs")
    });
}

#[test]
fn a_file_changed_on_disk_is_never_opened_from_what_was_read_before() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let file = std::path::Path::new(&dir).join("b.rs");
    std::fs::write(&file, "fn b() {}\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().iter().any(|f| f.path == "b.rs")
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: 0..40 }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("b.rs")
    });

    std::fs::write(&file, "fn b() { edited() }\n").unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .changes
            .get("b.rs")
            .is_some_and(|f| f.texts().any(|line| line.contains("edited")))
    });
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "b.rs".into(),
            at: None,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.active().is_some()
    });
    let open = state.workspace.active().expect("open");
    assert!(
        open.new
            .document()
            .line(0)
            .is_some_and(|l| l.contains("edited")),
        "the buffer holds what is on disk"
    );
}

/// A worktree whose change is committed, read against the branch it left: a review.
fn reviewed(
    state: &mut crate::AppState,
    services: &crate::Services,
    spawner: &SyncSpawner,
) -> String {
    let dir = worktree(state, services, spawner);
    let at = std::path::Path::new(&dir);
    let whole: String = (0..60).map(|n| format!("line {n}\n")).collect();
    std::fs::write(at.join("a.txt"), &whole).unwrap();
    sh(at, &["add", "-A"]);
    sh(at, &["commit", "-m", "the whole file"]);
    sh(at, &["push", "-q", "origin", "HEAD:refs/heads/review-base"]);
    sh(at, &["fetch", "-q", "origin"]);
    super::diff::based(state, "review-base");
    let changed = whole
        .replace("line 0\n", "LINE 0\n")
        .replace("line 59\n", "LINE 59\n");
    std::fs::write(at.join("a.txt"), changed).unwrap();
    sh(at, &["commit", "-qam", "the change under review"]);
    dir
}

#[test]
fn a_gap_in_a_committed_change_keeps_the_change_it_stands_in() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    reviewed(&mut state, &services, &spawner);
    state.workspace.mode = groove_types::DiffMode::Base;
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.pending.is_empty() && s.workspace.changes.get("a.txt").is_some()
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: 0..80 }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("a.txt")
    });
    let changed = |state: &crate::AppState| {
        state.workspace.changes.get("a.txt").map(|f| {
            f.rows()
                .filter(|row| {
                    !matches!(
                        row.kind,
                        groove_types::RowKind::Context | groove_types::RowKind::Gap(_)
                    )
                })
                .count()
        })
    };
    let before = changed(&state);
    assert_eq!(before, Some(4), "two lines out, two in");

    let gap = gap_row(&state).expect("a gap between the two changes");
    dispatch(
        Cmd::Workspace(workspace::Command::OpenGap {
            row: gap,
            way: workspace::Way::Down,
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(changed(&state), before, "the change is still there");
}

#[test]
fn a_committed_change_opened_and_edited_keeps_what_it_changed() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    reviewed(&mut state, &services, &spawner);
    state.workspace.mode = groove_types::DiffMode::Base;
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.pending.is_empty() && s.workspace.changes.get("a.txt").is_some()
    });
    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: 0..80 }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("a.txt")
    });
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "a.txt".into(),
            at: None,
        }),
        &mut state,
        &services,
        &spawner,
    );
    let open = state.workspace.active().expect("opened from memory");
    assert_eq!(
        open.old.line(0).as_deref(),
        Some("line 0"),
        "the old side is the branch it left, not HEAD"
    );

    dispatch(
        Cmd::Workspace(workspace::Command::Edit(Edit::Insert("x".into()))),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    let open = state.workspace.active().expect("still open");
    let changed = open
        .hunked
        .layout
        .all()
        .filter(|row| {
            !matches!(
                row.kind,
                groove_types::RowKind::Context | groove_types::RowKind::Gap(_)
            )
        })
        .count();
    assert!(
        changed >= 4,
        "the change is still there after an edit: {changed}"
    );
}
