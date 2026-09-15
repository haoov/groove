use groove_controllers::AppState;
use groove_types::{PoolEntry, SessionId};

use crate::input::{Key, Modifiers};
use crate::palette::{Palette, entries, matching};
use crate::tests::{app, full_app, metrics, press, with_repo};
use crate::{Ui, view};

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
    press(Key::Char('p'), chord, &mut ui, &app);
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let texts: Vec<String> = frame.layers()[1]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Add worktree"));
    for c in "close work".chars() {
        press(Key::Char(c), Modifiers::default(), &mut ui, &app);
    }
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let (frame, _) = view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
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
    let run = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(run[0].id(), "session.close_worktree");
    assert!(ui.palette.is_none());
}

#[test]
fn the_overview_lists_repos_and_worktrees_with_the_selected_one_marked() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
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
    assert!(texts.iter().any(|t| t == "REPOS AND WORKTREES"));
    assert!(texts.iter().any(|t| t == "mayo"));
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the header and the row"
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(
        icons.contains(&groove_gfx::Icon::Cube),
        "the repo wears a mark"
    );
    assert!(
        icons.contains(&groove_gfx::Icon::Compass),
        "the session's kind"
    );
}

#[test]
fn a_worktrees_counts_show_as_icons_and_zeros_do_not() {
    let mut app = full_app();
    let open = app.session.get_mut(&SessionId::new("a")).unwrap();
    let worktree = open.worktrees[0].id.clone();
    open.delivery.push((
        worktree,
        groove_types::WorktreeDelivery {
            status: groove_types::WorktreeStatus {
                modified: 3,
                staged: 0,
                ahead: 1,
                behind: 0,
            },
            ..Default::default()
        },
    ));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(icons.contains(&groove_gfx::Icon::ArrowUp), "ahead 1");
    assert!(icons.contains(&groove_gfx::Icon::Dot), "modified 3");
    assert!(
        !icons.contains(&groove_gfx::Icon::Plus),
        "staged 0 is not drawn"
    );
    assert!(
        !icons.contains(&groove_gfx::Icon::ArrowDown),
        "behind 0 is not drawn"
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "3"));
    assert!(texts.iter().any(|t| t == "1"));
}

#[test]
fn the_header_names_the_session_and_what_it_points_at() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
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
    assert!(texts.iter().any(|t| t == "Alpha"), "the title");
    assert!(texts.iter().any(|t| t == "mayo"), "the repo picker");
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the worktree picker and the overview row"
    );
    let carets = frame.layers()[0]
        .icons
        .iter()
        .filter(|i| i.icon == groove_gfx::Icon::CaretDown)
        .count();
    assert_eq!(carets, 2, "one per picker");
}

#[test]
fn a_session_without_a_worktree_says_so() {
    let mut app = app();
    app.session.selected = Some(SessionId::new("a"));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
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
    assert!(texts.iter().any(|t| t == "no repo"));
    assert!(texts.iter().any(|t| t == "no worktree"));
}

#[test]
fn the_header_stands_over_the_workspace_only() {
    let tokens = crate::tokens::Tokens::new(1.0);
    let layout = crate::layout::Layout::new(
        groove_gfx::Size::new(1280, 800),
        &tokens,
        crate::Split::default(),
    );
    assert_eq!(layout.agent.y, 0.0, "the agent pane runs full height");
    assert_eq!(layout.agent.h, 800.0);
    assert_eq!(layout.header.x, layout.agent.right());
    assert_eq!(layout.header.w, layout.workspace.w);
    assert_eq!(layout.header.h, tokens.header);
    assert_eq!(layout.workspace.y, layout.header.bottom());
    assert_eq!(layout.rail.right(), layout.agent.x);
}
