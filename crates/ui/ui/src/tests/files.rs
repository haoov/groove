use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus};

mod commits;
mod explorer;
mod keys;
mod panes;
mod paths;
mod tabs;

use crate::hit::Target;
use crate::input::Key;
use crate::layout::{Layout, Split};
use crate::tests::{ALT_SHIFT, click, full_app, press, render, window};
use crate::views::session::Mode;
use crate::{Ui, view};
use groove_ui_kit::base::tokens::Tokens;

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
    ui.session.mode = Mode::Diff;
    ui.split.sidebar = crate::tests::WIDE_SIDEBAR;
    ui
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

/// Every text the sidebar drew.
fn sidebar_texts(frame: &groove_gfx::Frame, ui: &Ui) -> Vec<(String, groove_gfx::Color)> {
    let sidebar = Layout::of(window(), ui).sidebar;
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .filter(|run| run.x >= sidebar.x)
        .map(|run| (run.text.clone(), run.style.color))
        .collect()
}

#[test]
fn a_row_names_its_file_alone_its_letter_in_its_status_colour() {
    let app = with_files(&[
        "crates/ui/ui/src/views/session/session.rs",
        "crates/ui/ui/src/tokens.rs",
    ]);
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = sidebar_texts(&frame, &ui);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let warn = styles.color(groove_ui_kit::base::style::Role::Warn);
    assert!(
        texts
            .iter()
            .any(|(text, color)| text == "M" && *color == warn),
        "{texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|(text, _)| text == "crates/ui/ui/src/views/session"),
        "the directory row says the path, the file row does not: {texts:?}"
    );
}

#[test]
fn a_clean_worktree_says_so_in_both_places() {
    let app = full_app();
    let (texts, _) = render(&app, &on_diff(), window());
    assert!(texts.iter().any(|t| t == "nothing changed"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "No change in this worktree."));
}

#[test]
fn files_loaded_for_another_worktree_are_never_shown() {
    let mut app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    app.workspace.worktree = Some(groove_types::WorktreeId::new("somewhere-else"));
    let ui = on_diff();
    let (texts, _) = render(&app, &ui, window());
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
    let fold = hits.rect_of(&Target::AsideFold).expect("the fold");
    assert!(click(fold, &mut ui, &app, &hits).is_empty());
    assert!(click(fold, &mut ui, &app, &hits).is_empty());
    assert!(press(Key::Char('e'), crate::tests::ALT, &mut ui, &app).is_empty());
}

#[test]
fn a_chord_reads_the_worktree_whenever_the_user_asks() {
    let app = with_files(&["crates/ui/ui/src/tokens.rs"]);
    let mut ui = on_diff();
    let asked = press(Key::Char('r'), ALT_SHIFT, &mut ui, &app);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].id(), "workspace.load");
}

#[test]
fn the_band_says_what_the_change_is_read_against_and_picks_one() {
    let app = with_files(&["a.txt"]);
    let mut ui = on_diff();
    ui.split.sidebar = Split::default().sidebar;
    let (drawn, hits) = render(&app, &ui, window());
    for mode in groove_types::DiffMode::ALL {
        assert!(
            drawn.iter().any(|one| *one == mode.label().to_uppercase()),
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

/// What a pane's tab of the Diff's sidebar says, its count after its name; none when not drawn.
fn tab_said(app: &AppState, ui: &Ui, pane: crate::views::session::Pane) -> Option<String> {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let tab = hits.rect_of(&Target::Pane(pane))?;
    let said = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|run| tab.contains(run.x, run.y + run.height / 2.0))
        .map_or(String::new(), |run| run.text.clone());
    Some(said)
}

#[test]
fn a_row_under_the_pointer_and_the_open_one_read_apart() {
    let app = with_files(&["a.txt"]);
    let mut ui = on_diff();
    let tokens = Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let grounds = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let line = hits
            .rect_of(&Target::File("a.txt".into()))
            .expect("the row");
        let quads: Vec<groove_gfx::Quad> = frame
            .layers()
            .iter()
            .flat_map(|layer| layer.quads.iter().cloned())
            .collect();
        let of = |color| {
            quads
                .iter()
                .filter(|quad| quad.color == color && quad.rect.h > tokens.row / 2.0)
                .filter(|quad| line.contains(quad.rect.x, quad.rect.y + 1.0))
                .count()
        };
        (of(styles.glass_hover()), of(styles.glass_chosen()))
    };
    assert_eq!(grounds(&ui), (0, 0), "a row at rest carries neither");
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(grounds(&ui).0, 1, "the pointer grounds the row");
    assert_ne!(
        styles.glass_hover(),
        styles.glass_chosen(),
        "and the two never match"
    );
    assert_ne!(
        styles.hover(),
        styles.action(),
        "nor does a button's ground"
    );
}
