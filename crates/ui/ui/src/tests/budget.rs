//! What a frame is allowed to cost: the window it draws, never the file behind it.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{DiffView, FileDiff, FileStatus};

use crate::tests::{full_app, metrics, shows};
use crate::{Metrics, Ui, view};

/// A file of `lines` identical lines with the twentieth changed, so every view has
/// rows and no row differs from another.
fn file(lines: usize) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace.loaded(
        worktree,
        vec![FileDiff {
            path: "src/lib.rs".into(),
            added: 1,
            deleted: 1,
            status: FileStatus::Modified,
            staged: Some(false),
        }],
        Default::default(),
    );
    let mut rows = vec!["let value = one();"; lines];
    let before = rows.join("\n") + "\n";
    rows[19] = "let value = two();";
    let after = rows.join("\n") + "\n";
    shows(&mut app, "src/lib.rs", &before, &after);
    app
}

/// How many text runs a frame of that file draws.
fn runs(app: &AppState, face: crate::views::session::Face, window: Metrics) -> usize {
    let mut ui = Ui::default();
    crate::tests::set_face(&mut ui, face);
    let (frame, _) = view(app, &ui, window, &mut Fonts::embedded());
    frame.layers().iter().map(|layer| layer.texts.len()).sum()
}

fn window() -> Metrics {
    metrics(1280, 800, 1.0)
}

#[test]
fn a_frame_draws_the_window_not_the_file() {
    let (small, huge) = (file(200), file(20_000));
    let faces = DiffView::ALL.map(crate::views::session::Face::Stream);
    for view_kind in [crate::views::session::Face::File].into_iter().chain(faces) {
        assert_eq!(
            runs(&huge, view_kind, window()),
            runs(&small, view_kind, window()),
            "{view_kind:?} draws a hundred times the file for the same price"
        );
    }
}

#[test]
fn a_taller_window_draws_more_rows() {
    let app = file(20_000);
    let short = runs(
        &app,
        crate::views::session::Face::File,
        metrics(1280, 400, 1.0),
    );
    let tall = runs(&app, crate::views::session::Face::File, window());
    assert!(
        tall > short,
        "800px draws more rows than 400px: {tall} against {short}"
    );
}
