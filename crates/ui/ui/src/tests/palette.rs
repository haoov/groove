use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::{
    PoolEntry, Repo, RepoId, Session, SessionId, SessionKind, SessionState, Timestamp, Worktree,
    WorktreeId,
};

use crate::input::{Input, Key, Modifiers, handle};
use crate::palette::{Palette, entries, matching};
use crate::{Metrics, Ui, view};

fn open(id: &str, title: &str) -> Open {
    Open {
        session: Session {
            id: SessionId::new(id),
            title: title.into(),
            kind: SessionKind::Explorer,
            created_at: Timestamp::new(0),
        },
        state: SessionState::default(),
        repos: vec![],
        worktrees: vec![],
        delivery: vec![],
    }
}

fn app() -> AppState {
    let mut app = AppState::default();
    app.session.open.push(open("a", "Alpha"));
    app.session.open.push(open("b", "Beta"));
    app.session.selected = Some(SessionId::new("a"));
    app.session.pool = vec![PoolEntry {
        slug: "gitlab.example.com/g/mayo".into(),
        path: "/pool/mayo".into(),
    }];
    app
}

fn with_repo(mut app: AppState) -> AppState {
    let repo = Repo {
        id: RepoId::new("gitlab.example.com/g/mayo"),
        host: "gitlab.example.com".into(),
        group_path: "g".into(),
        project: "mayo".into(),
        local_path: "/pool/mayo".into(),
    };
    let wt = Worktree {
        id: WorktreeId::new("wt-1"),
        session: SessionId::new("a"),
        repo: repo.id.clone(),
        branch: "explorer/alpha".into(),
        path: "/code/worktrees/a/mayo/explorer/alpha".into(),
        base_ref: None,
        created_at: Timestamp::new(0),
    };
    app.session
        .branches
        .push((repo.id.clone(), vec!["main".into(), "release/1.0".into()]));
    app.session
        .get_mut(&SessionId::new("a"))
        .unwrap()
        .add_worktree(repo, wt);
    app
}

fn typed(palette: &mut Palette, text: &str, app: &AppState) {
    for c in text.chars() {
        palette.key(Key::Char(c), app);
    }
}

fn labels(app: &AppState) -> Vec<String> {
    entries(app).into_iter().map(|e| e.label).collect()
}

#[test]
fn entries_follow_what_the_session_holds() {
    assert_eq!(labels(&AppState::default()), ["New explorer"]);
    let bare = labels(&app());
    assert_eq!(
        bare,
        [
            "New explorer",
            "Add repo",
            "Rename explorer",
            "Delete session",
            "Close session",
            "Switch to Beta"
        ]
    );
    let full = labels(&with_repo(app()));
    assert!(
        full.contains(&"Add worktree".to_string())
            && full.contains(&"Remove repo".to_string())
            && full.contains(&"Close worktree".to_string())
    );
    assert!(
        !full.contains(&"Select worktree".to_string()),
        "one worktree: nothing to select between"
    );
    assert!(entries(&full_app()).iter().all(|e| e.id().contains('.')));
}

fn full_app() -> AppState {
    with_repo(app())
}

#[test]
fn the_query_filters_and_ranks() {
    let rows = matching(entries(&app()), |e| e.label.clone(), "beta");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "Switch to Beta");
    assert!(matching(entries(&app()), |e| e.label.clone(), "zzz").is_empty());
}

#[test]
fn add_repo_picks_from_the_pool_or_clones_from_a_url() {
    let mut app = full_app();
    app.session.pool.push(PoolEntry {
        slug: "github.com/o/other".into(),
        path: "/pool/other".into(),
    });
    let mut palette = Palette::default();
    typed(&mut palette, "add repo", &app);
    let opened = palette.key(Key::Enter, &app);
    assert!(!opened.close);
    assert_eq!(
        opened.commands[0].id(),
        "session.list_repos",
        "the pool is refreshed first"
    );
    let prompt = palette.prompt(&app).unwrap();
    assert_eq!(prompt.label, "repo");
    assert!(!prompt.free, "a picker, not a field");
    let labels: Vec<&str> = prompt.options.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        labels,
        ["github.com/o/other", "Clone from a URL…"],
        "the held repo is out, the clone row is in"
    );

    typed(&mut palette, "zzz", &app);
    let rows = palette.options(&app);
    assert_eq!(rows.len(), 1, "nothing matches: only the clone row");
    assert_eq!(rows[0].0, "Clone from a URL…");
    palette.key(Key::Enter, &app);
    assert_eq!(palette.prompt(&app).unwrap().label, "git URL");
    assert!(
        palette.key(Key::Enter, &app).commands.is_empty(),
        "a URL is required"
    );
    typed(&mut palette, "git@github.com:o/new.git", &app);
    palette.key(Key::Enter, &app);
    assert_eq!(
        palette.prompt(&app).unwrap().label,
        "branch, empty for the default"
    );
    palette.key(Key::Enter, &app);
    assert!(palette.prompt(&app).unwrap().allow_empty);
    let done = palette.key(Key::Enter, &app);
    assert!(done.close);
    match &done.commands[0] {
        groove_controllers::Command::Session(groove_controllers::session::Command::AddRepo {
            name,
            spec,
            ..
        }) => {
            assert_eq!(name, "git@github.com:o/new.git");
            assert_eq!(
                (spec.branch.as_deref(), spec.target.as_deref()),
                (None, None)
            );
        }
        other => panic!("{other:?}"),
    }

    let mut palette = Palette::default();
    typed(&mut palette, "add repo", &app);
    palette.key(Key::Enter, &app);
    typed(&mut palette, "oth", &app);
    palette.key(Key::Enter, &app);
    assert_eq!(
        palette.prompt(&app).unwrap().label,
        "branch, empty for the default",
        "a picked repo skips the URL"
    );
    typed(&mut palette, "feat/x", &app);
    palette.key(Key::Enter, &app);
    let done = palette.key(Key::Enter, &app);
    match &done.commands[0] {
        groove_controllers::Command::Session(groove_controllers::session::Command::AddRepo {
            name,
            spec,
            ..
        }) => {
            assert_eq!(name, "github.com/o/other");
            assert_eq!(spec.branch.as_deref(), Some("feat/x"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn add_worktree_takes_a_typed_branch_and_a_picked_base_and_escape_backs_out() {
    let app = full_app();
    let mut palette = Palette::default();
    typed(&mut palette, "add work", &app);
    palette.key(Key::Enter, &app);
    assert_eq!(palette.prompt(&app).unwrap().label, "repo");
    palette.key(Key::Enter, &app);
    assert_eq!(palette.prompt(&app).unwrap().label, "branch");
    assert!(
        palette.key(Key::Enter, &app).commands.is_empty(),
        "a required branch refuses empty"
    );
    typed(&mut palette, "fix/x", &app);
    palette.key(Key::Enter, &app);
    let base = palette.prompt(&app).unwrap();
    assert_eq!(base.options.len(), 2, "origin's heads were already listed");
    typed(&mut palette, "rel", &app);
    let done = palette.key(Key::Enter, &app);
    assert!(done.close);
    match &done.commands[0] {
        groove_controllers::Command::Session(
            groove_controllers::session::Command::AddWorktree { repo, spec, .. },
        ) => {
            assert_eq!(repo.as_str(), "gitlab.example.com/g/mayo");
            assert_eq!(spec.branch.as_deref(), Some("fix/x"));
            assert_eq!(spec.target.as_deref(), Some("release/1.0"));
        }
        other => panic!("{other:?}"),
    }

    let mut palette = Palette::default();
    typed(&mut palette, "rename", &app);
    palette.key(Key::Enter, &app);
    typed(&mut palette, "new name", &app);
    assert!(
        !palette.key(Key::Escape, &app).close,
        "escape in a flow leaves the flow, not the palette"
    );
    assert!(palette.flow.is_none());
    assert!(palette.key(Key::Escape, &app).close);
}

#[test]
fn the_palette_draws_a_prompt_and_closes_on_a_plain_command() {
    let app = full_app();
    let mut ui = Ui::default();
    let chord = Modifiers {
        ctrl: true,
        shift: true,
        alt: false,
    };
    handle(
        Input::Key {
            key: Key::Char('p'),
            mods: chord,
        },
        &mut ui,
        &app,
    );
    let metrics = Metrics {
        size: groove_gfx::Size::new(1280, 800),
        scale: 1.0,
        cell: groove_gfx::CellSize {
            width: 8.0,
            height: 17.0,
        },
        tick: 0,
    };
    let frame = view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Add worktree"));
    for c in "close work".chars() {
        handle(
            Input::Key {
                key: Key::Char(c),
                mods: Modifiers::default(),
            },
            &mut ui,
            &app,
        );
    }
    handle(
        Input::Key {
            key: Key::Enter,
            mods: Modifiers::default(),
        },
        &mut ui,
        &app,
    );
    let frame = view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("worktree: ")),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "mayo · explorer/alpha"));
    let run = handle(
        Input::Key {
            key: Key::Enter,
            mods: Modifiers::default(),
        },
        &mut ui,
        &app,
    );
    assert_eq!(run[0].id(), "session.close_worktree");
    assert!(ui.palette.is_none());
}

#[test]
fn the_overview_lists_repos_and_worktrees_with_the_selected_one_marked() {
    let app = full_app();
    let metrics = Metrics {
        size: groove_gfx::Size::new(1280, 800),
        scale: 1.0,
        cell: groove_gfx::CellSize {
            width: 8.0,
            height: 17.0,
        },
        tick: 0,
    };
    let frame = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "overview"));
    assert!(texts.iter().any(|t| t == "mayo"));
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the header and the row"
    );
}
