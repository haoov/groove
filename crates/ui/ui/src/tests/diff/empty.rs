//! A file with nothing in it: the editor still has the one line a caret stands on.

use super::*;
use crate::views::session::diff;

/// The fixture's worktree with an empty file open in the editor.
fn nothing() -> (AppState, Ui) {
    let mut app = with_files();
    app.workspace.opened = Some(groove_controllers::workspace_service::from_text(
        "src/new.rs",
        "",
        "",
    ));
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    ui.focus = crate::Focus::Workspace;
    (app, ui)
}

#[test]
fn the_editor_stands_one_row_tall_on_an_empty_file() {
    let (app, ui) = nothing();
    assert_eq!(diff::rows_of(&app, &ui), 1, "there is a line to type on");
    assert_eq!(
        diff::line_at(&app, &ui, DiffView::Editor, 0),
        Some(("src/new.rs".to_string(), 0)),
        "and a click on it lands on that line"
    );
}

#[test]
fn the_caret_shows_on_the_one_line_of_an_empty_file() {
    let (app, ui) = nothing();
    let styles = crate::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let bar = Tokens::new(1.0).hairline * 2.0;
    let carets = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.caret() && quad.rect.w == bar)
        .filter(|quad| quad.rect.x >= workspace.x && quad.rect.y >= workspace.y)
        .count();
    assert_eq!(
        carets, 1,
        "the caret stands on the code surface, so there is somewhere to type"
    );
}
