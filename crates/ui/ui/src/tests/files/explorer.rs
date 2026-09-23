//! The explorer: what the tree shows, and what opening a directory adds to it.

use super::search::with_paths;
use super::*;
use crate::views::session::Scope;

/// The sidebar on the whole worktree, with these paths walked.
pub(super) fn browsing(paths: &[&str]) -> (AppState, Ui) {
    let mut app = with_files(&[]);
    with_paths(&mut app, paths);
    let mut ui = on_diff();
    ui.session.scope = Scope::All;
    (app, ui)
}

fn shown(app: &AppState, ui: &Ui) -> Vec<String> {
    let sidebar = Layout::of(window(), ui).sidebar;
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= sidebar.x)
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_tree_shows_what_stands_at_the_root_and_nothing_under_it() {
    let (app, ui) = browsing(&["src/one/alpha.rs", "src/two/beta.rs", "README.md"]);
    let drawn = shown(&app, &ui);
    assert!(drawn.iter().any(|t| t == "src"), "the directory: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "README.md"), "{drawn:?}");
    assert!(
        !drawn.iter().any(|t| t == "one" || t == "alpha.rs"),
        "nothing inside it while it is shut: {drawn:?}"
    );
}

#[test]
fn opening_a_directory_shows_what_it_holds() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs", "README.md"]);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Dir("src".into()))
        .expect("the directory is there to open");
    let asked = crate::input::handle(
        crate::input::Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert!(asked.is_empty(), "opening one asks nothing of the worktree");
    assert!(ui.session.opened.contains("src"));
    let drawn = shown(&app, &ui);
    assert!(drawn.iter().any(|t| t == "one"), "{drawn:?}");
    assert!(
        !drawn.iter().any(|t| t == "alpha.rs"),
        "and only one level of it: {drawn:?}"
    );
}

#[test]
fn a_file_of_the_tree_that_changed_keeps_its_marks() {
    let mut app = with_files(&["src/one/alpha.rs"]);
    with_paths(&mut app, &["src/one/alpha.rs"]);
    let mut ui = on_diff();
    ui.session.scope = Scope::All;
    ui.session.opened.insert("src".into());
    ui.session.opened.insert("src/one".into());
    let drawn = shown(&app, &ui);
    assert!(drawn.iter().any(|t| t == "alpha.rs"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "M"), "its status: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "+2"), "its counts: {drawn:?}");
}

#[test]
fn picking_the_whole_worktree_switches_the_scope_and_the_frame_asks_for_the_walk() {
    let app = with_files(&[]);
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Scope(Scope::All))
        .expect("the scope is there to pick");
    let asked = crate::input::handle(
        crate::input::Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(ui.session.scope, Scope::All);
    assert!(asked.is_empty(), "the click itself asks nothing");
    assert!(
        crate::views::session::files::needs_walk(&app, &ui),
        "and the frame sees that the tree needs a walk"
    );

    let mut walked = with_files(&[]);
    with_paths(&mut walked, &["README.md"]);
    assert!(
        !crate::views::session::files::needs_walk(&walked, &ui),
        "the walk it already has is kept"
    );
}

#[test]
fn a_path_typed_while_browsing_flattens_the_tree_to_its_matches() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs", "src/two/beta.rs"]);
    ui.session.bar.path.set("beta");
    let drawn = shown(&app, &ui);
    assert!(drawn.iter().any(|t| t == "beta.rs"), "{drawn:?}");
    assert!(
        !drawn.iter().any(|t| t == "src"),
        "no tree while a query narrows it: {drawn:?}"
    );
}
