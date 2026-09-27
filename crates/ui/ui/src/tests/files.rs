use groove_controllers::AppState;
use groove_gfx::{Fonts, Size};
use groove_types::{FileDiff, FileStatus};

mod commits;
mod explorer;
mod panes;
mod paths;
mod search;

use crate::base::hit::Target;
use crate::base::tokens::Tokens;
use crate::input::Key;
use crate::layout::{Layout, Split};
use crate::tests::{CHORD, WINDOW, click, full_app, press, window};
use crate::views::session::Tab;
use crate::views::session::files::listing;
use crate::{Ui, view};

fn changed(path: &str, added: u32, deleted: u32) -> FileDiff {
    FileDiff {
        path: path.into(),
        added,
        deleted,
        status: FileStatus::Modified,
        staged: Some(false),
    }
}

fn with_files(paths: &[&str]) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace.loaded(
        worktree,
        paths.iter().map(|p| changed(p, 2, 1)).collect(),
        Default::default(),
    );
    app
}

fn on_diff() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui
}

fn ctrl() -> crate::input::Modifiers {
    crate::input::Modifiers {
        ctrl: true,
        ..crate::input::Modifiers::default()
    }
}

#[test]
fn a_file_is_grouped_by_its_directory_from_the_worktree_root() {
    let files = [
        changed("crates/ui/ui/src/views/session/session.rs", 1, 0),
        changed("crates/ui/ui/src/views/shared/rail.rs", 1, 0),
        changed("crates/ui/ui/src/tokens.rs", 1, 0),
        changed("Cargo.toml", 1, 0),
    ];
    let listing = listing(&files.iter().collect::<Vec<_>>());
    let groups: Vec<(&str, Vec<&str>)> = listing
        .groups
        .iter()
        .map(|g| {
            (
                g.dir.as_str(),
                g.files.iter().map(|f| f.path.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(listing.root, "", "nothing is shared with a file at the top");
    assert_eq!(
        groups,
        [
            ("", vec!["Cargo.toml"]),
            ("crates/ui/ui/src", vec!["crates/ui/ui/src/tokens.rs"]),
            (
                "crates/ui/ui/src/views/session",
                vec!["crates/ui/ui/src/views/session/session.rs"]
            ),
            (
                "crates/ui/ui/src/views/shared",
                vec!["crates/ui/ui/src/views/shared/rail.rs"]
            ),
        ],
        "the worktree root is the only root"
    );
}

#[test]
fn what_every_file_shares_is_shown_once_and_taken_off_the_groups() {
    let files = [
        changed("crates/ui/ui/src/tokens.rs", 1, 0),
        changed("crates/ui/ui/src/views/session/session.rs", 1, 0),
        changed("crates/ui/ui/src/views/shared/rail.rs", 1, 0),
    ];
    let listing = listing(&files.iter().collect::<Vec<_>>());
    assert_eq!(listing.root, "crates/ui/ui/src");
    let dirs: Vec<&str> = listing.groups.iter().map(|g| g.dir.as_str()).collect();
    assert_eq!(
        dirs,
        ["", "views/session", "views/shared"],
        "and a chain with one child is one line, not three"
    );
}

#[test]
fn a_name_that_says_nothing_reads_as_its_directory() {
    use crate::views::session::files::reads_as;

    assert_eq!(
        reads_as("crates/ui/ui/src/views/session/mod.rs"),
        (
            "session/mod.rs".to_string(),
            "crates/ui/ui/src/views".to_string()
        )
    );
    assert_eq!(
        reads_as("crates/ui/ui/src/tokens.rs"),
        ("tokens.rs".to_string(), "crates/ui/ui/src".to_string())
    );
    assert_eq!(
        reads_as("Cargo.toml"),
        ("Cargo.toml".to_string(), String::new())
    );
}

#[test]
fn one_file_is_all_root_and_no_group_line() {
    let files = [changed("crates/base/gfx/src/icons.rs", 3, 2)];
    let listing = listing(&files.iter().collect::<Vec<_>>());
    assert_eq!(listing.root, "crates/base/gfx/src");
    assert_eq!(listing.groups.len(), 1);
    assert_eq!(listing.groups[0].dir, "", "its path is the root itself");
}

#[test]
fn the_sidebar_stands_beside_the_workspace_only_where_a_tab_wants_it() {
    let tokens = Tokens::new(1.0);
    let size = Size::new(WINDOW.0, WINDOW.1);
    let folded = Layout::new(size, &tokens, Split::default(), false);
    assert!(folded.sidebar.is_empty());
    assert_eq!(folded.workspace.right(), folded.window.right());

    let open = Layout::new(size, &tokens, Split::default(), true);
    assert_eq!(open.sidebar.w, Split::default().sidebar);
    assert_eq!(open.sidebar.x, open.workspace.right());
    assert_eq!(open.sidebar.right(), open.window.right());
    assert!(
        open.workspace.w < folded.workspace.w,
        "the workspace gives the room, not the agent"
    );
    assert_eq!(open.agent.x, folded.agent.x);
}

#[test]
fn the_files_tab_names_the_files_with_what_they_changed() {
    let app = with_files(&[
        "crates/ui/ui/src/views/session/session.rs",
        "crates/ui/ui/src/tokens.rs",
    ]);
    let (frame, _) = view(&app, &on_diff(), window(), &mut Fonts::embedded());
    let sidebar = Layout::of(window(), &on_diff()).sidebar;
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= sidebar.x)
        .map(|run| run.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "changed · 2"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "all"),
        "the other scope: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "tokens.rs"));
    assert!(texts.iter().any(|t| t == "session.rs"));
    assert!(
        texts.iter().any(|t| t == "crates/ui/ui/src"),
        "the root, once: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "views/session"),
        "and a group under it"
    );
    assert!(texts.iter().any(|t| t == "+2"));
    assert!(texts.iter().any(|t| t == "-1"));
    assert!(
        !texts.iter().any(|t| t.contains("views/session/session.rs")),
        "the group carries the path, the row carries the name"
    );
}

#[test]
fn a_clean_worktree_says_so_in_both_places() {
    let app = full_app();
    let (frame, _) = view(&app, &on_diff(), window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "nothing changed"));
    assert!(texts.iter().any(|t| t == "No change in this worktree."));
}

#[test]
fn folding_the_sidebar_gives_its_room_to_the_workspace_and_no_one_else() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let open = Layout::of(window(), &ui);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let fold = hits
        .rect_of(&Target::Fold)
        .expect("the strip carries the fold");

    assert!(click(fold, &mut ui, &app, &hits).is_empty());
    assert!(ui.session.folded);
    let folded = Layout::of(window(), &ui);
    assert!(folded.sidebar.is_empty());
    assert_eq!(folded.rail, open.rail);
    assert_eq!(folded.agent, open.agent, "the agent pane never moves");
    assert_eq!(folded.workspace.w, open.workspace.w + open.sidebar.w);

    press(Key::Char('b'), CHORD, &mut ui, &app);
    assert!(!ui.session.folded, "the chord brings it back");
    assert_eq!(Layout::of(window(), &ui).sidebar.w, open.sidebar.w);
}

#[test]
fn a_tab_with_no_sidebar_has_nothing_to_fold() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::Fold).is_none());
}

#[test]
fn files_loaded_for_another_worktree_are_never_shown() {
    let mut app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    app.workspace.worktree = Some(groove_types::WorktreeId::new("somewhere-else"));
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect();
    assert!(
        texts.iter().any(|t| t == "nothing changed"),
        "a summary of another worktree is not this worktree's: {texts:?}"
    );
    assert!(!texts.iter().any(|t| t == "tokens.rs"));
}

#[test]
fn folding_the_sidebar_asks_nothing_of_the_worktree() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let fold = hits.rect_of(&Target::Fold).expect("the fold");
    assert!(click(fold, &mut ui, &app, &hits).is_empty());
    assert!(click(fold, &mut ui, &app, &hits).is_empty());
    assert!(press(Key::Char('b'), CHORD, &mut ui, &app).is_empty());
}

#[test]
fn a_chord_reads_the_worktree_whenever_the_user_asks() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let asked = press(Key::Char('r'), CHORD, &mut ui, &app);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].id(), "workspace.load");
}

#[test]
fn a_row_reads_as_its_name_with_the_rest_of_its_path_behind_it() {
    let app = with_files(&[
        "crates/ui/ui/src/tokens.rs",
        "crates/other/src/views/session/mod.rs",
    ]);
    let mut ui = on_diff();
    ui.session.tab = Tab::File;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let sidebar = Layout::of(window(), &ui).sidebar;
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= sidebar.x)
        .map(|run| run.text.clone())
        .collect();
    assert!(
        texts.iter().any(|t| t == "session/mod.rs"),
        "the directory names it: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "tokens.rs"));
    assert!(
        !texts.iter().any(|t| t == "mod.rs"),
        "never a bare mod.rs, which says nothing"
    );
}

#[test]
fn the_strip_names_the_two_tabs_the_workspace_has() {
    let app = with_files(&["src/one/alpha.rs"]);
    let (frame, _) = view(&app, &on_diff(), window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    assert!(texts.iter().any(|one| one == "overview"), "{texts:?}");
    assert!(texts.iter().any(|one| one == "file"), "{texts:?}");
    assert!(!texts.iter().any(|one| one == "diff"), "{texts:?}");
}

#[test]
fn the_list_says_what_the_change_is_read_against_and_picks_one() {
    let app = with_files(&["a.txt"]);
    let mut ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect();
    for mode in groove_types::DiffMode::ALL {
        assert!(
            drawn.iter().any(|one| one == mode.label()),
            "{mode:?}: {drawn:?}"
        );
        assert!(hits.rect_of(&Target::Mode(mode)).is_some(), "{mode:?}");
    }

    let base = hits
        .rect_of(&Target::Mode(groove_types::DiffMode::Base))
        .expect("the base mode");
    assert_eq!(
        click(base, &mut ui, &app, &hits),
        [groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::SetMode {
                mode: groove_types::DiffMode::Base
            }
        )]
    );
}

#[test]
fn the_rule_under_the_heading_runs_the_whole_width() {
    let app = with_files(&["a.txt"]);
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let mode = hits
        .rect_of(&Target::Mode(groove_types::DiffMode::Base))
        .expect("the base mode");
    let sidebar = Layout::of(window(), &ui).sidebar;
    let under = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.h <= 1.0 && quad.rect.x >= sidebar.x)
        .filter(|quad| quad.rect.y > mode.y && quad.rect.y <= mode.bottom())
        .max_by(|a, b| a.rect.w.total_cmp(&b.rect.w))
        .expect("a rule under the heading");
    assert!(
        under.rect.right() >= mode.right(),
        "it runs past the modes: {} against {}",
        under.rect.right(),
        mode.right()
    );
    assert!(under.rect.w >= sidebar.w - 1.0, "{}", under.rect.w);
}
