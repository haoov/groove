use groove_controllers::AppState;
use groove_gfx::{Fonts, Size};
use groove_types::{FileDiff, FileStatus};

use crate::hit::Target;
use crate::input::Key;
use crate::layout::{Layout, Split};
use crate::tests::{CHORD, WINDOW, click, full_app, press, window};
use crate::tokens::Tokens;
use crate::views::session::Tab;
use crate::views::session::components::files::listing;
use crate::{Ui, view};

fn changed(path: &str, added: u32, deleted: u32) -> FileDiff {
    FileDiff {
        path: path.into(),
        added,
        deleted,
        status: FileStatus::Modified,
        staged: Some(false),
        hunks: Vec::new(),
    }
}

fn with_files(paths: &[&str]) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree, paths.iter().map(|p| changed(p, 2, 1)).collect());
    app
}

fn on_diff() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui
}

#[test]
fn the_root_every_path_shares_is_shown_once() {
    let files = [
        changed("crates/ui/ui/src/views/session/session.rs", 1, 0),
        changed("crates/ui/ui/src/views/shared/rail.rs", 1, 0),
        changed("crates/ui/ui/src/tokens.rs", 1, 0),
    ];
    let listing = listing(&files);
    assert_eq!(listing.root, "crates/ui/ui/src");
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
    assert_eq!(
        groups,
        [
            ("", vec!["crates/ui/ui/src/tokens.rs"]),
            (
                "views/session",
                vec!["crates/ui/ui/src/views/session/session.rs"]
            ),
            (
                "views/shared",
                vec!["crates/ui/ui/src/views/shared/rail.rs"]
            ),
        ],
        "a chain with one child costs one line, not six"
    );
}

#[test]
fn paths_that_share_nothing_keep_their_whole_directory() {
    let files = [
        changed("Cargo.toml", 1, 0),
        changed("docs/design/design.md", 1, 0),
    ];
    let listing = listing(&files);
    assert!(listing.root.is_empty());
    assert_eq!(listing.groups[0].dir, "");
    assert_eq!(listing.groups[1].dir, "docs/design");
}

#[test]
fn one_file_puts_its_directory_in_the_root() {
    let files = [changed("crates/base/gfx/src/icons.rs", 3, 2)];
    let listing = listing(&files);
    assert_eq!(listing.root, "crates/base/gfx/src");
    assert_eq!(listing.groups.len(), 1);
    assert_eq!(listing.groups[0].dir, "");
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
    assert!(texts.iter().any(|t| t == "FILES · 2"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "crates/ui/ui/src"));
    assert!(texts.iter().any(|t| t == "tokens.rs"));
    assert!(texts.iter().any(|t| t == "session.rs"));
    assert!(texts.iter().any(|t| t == "views/session"));
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
fn bringing_the_list_back_into_view_reads_the_worktree_again() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let fold = hits.rect_of(&Target::Fold).expect("the fold");

    assert!(
        click(fold, &mut ui, &app, &hits).is_empty(),
        "folding it away asks for nothing"
    );
    let back = click(fold, &mut ui, &app, &hits);
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].id(), "workspace.load", "unfolding reads it again");

    let mut on_overview = Ui::default();
    let opened = press(Key::Char('b'), CHORD, &mut on_overview, &app);
    assert!(opened.is_empty(), "the overview has no list to read");
}

#[test]
fn a_chord_reads_the_worktree_whenever_the_user_asks() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let asked = press(Key::Char('r'), CHORD, &mut ui, &app);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].id(), "workspace.load");
}
