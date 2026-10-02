mod across;
mod empty;
mod gaps;
mod header;
mod notes;
mod noting;
mod pinned;
mod pointer;
mod prose;
mod rows;
mod stream;
mod views;

use groove_controllers::AppState;
use groove_gfx::{Fonts, Rect};

use crate::components::code_at;
use crate::hit::Target;
use crate::input::{Delta, Input};
use crate::tests::{WINDOW, click, full_app, handle, shows, window};
use crate::views::session::Tab;
use crate::{Ui, view};
use groove_types::{DiffView, LineMark};
use groove_ui_kit::base::tokens::Tokens;

const OLD: &str = "fn one() {}\nfn two() {}\nfn three() {}\n";
const NEW: &str = "fn one() {}\nfn TWO() {}\nfn three() {}\n";

pub(super) fn opened() -> AppState {
    let mut app = with_files();
    shows(&mut app, "src/lib.rs", OLD, NEW);
    app
}

fn with_files() -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    let file = groove_types::FileDiff {
        path: "src/lib.rs".into(),
        added: 1,
        deleted: 1,
        status: groove_types::FileStatus::Modified,
        staged: Some(false),
    };
    app.workspace
        .loaded(worktree, vec![file], Default::default());
    app
}

fn on_diff() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui
}

/// Every text the open file's rows drew.
fn row_texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y))
        .map(|run| run.text.clone())
        .collect()
}

/// Every text the workspace drew.
fn texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let workspace = crate::layout::Layout::of(window(), ui).workspace;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= workspace.x && run.x < workspace.right())
        .map(|run| run.text.clone())
        .collect()
}

/// The rows of one view, as the surface drew them, both panes in split.
fn in_view(app: &AppState, face: crate::views::session::Face) -> Vec<String> {
    let mut ui = on_diff();
    crate::tests::set_face(&mut ui, face);
    let (frame, _) = view_of(app, &ui);
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let tokens = Tokens::new(1.0);
    let strip = match face {
        crate::views::session::Face::File => tokens.row + tokens.sm,
        _ => 0.0,
    };
    let body = workspace.y + tokens.row * 2.0 + strip;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= workspace.x && run.x < workspace.right() && run.y >= body)
        .map(|run| run.text.clone())
        .collect()
}

fn view_of(app: &AppState, ui: &Ui) -> (groove_gfx::Frame, crate::Hits) {
    view(app, ui, window(), &mut Fonts::embedded())
}

/// The marks the file view drew, by colour.
fn marks(app: &AppState) -> Vec<LineMark> {
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let (frame, _) = view_of(app, &ui);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let width = Tokens::new(1.0).hairline * 2.0;
    frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.w == width)
        .filter_map(|quad| {
            [LineMark::Added, LineMark::Removed, LineMark::Changed]
                .into_iter()
                .find(|mark| styles.mark(*mark) == quad.color)
        })
        .collect()
}

/// Thirty lines with one changed, far enough in for the alignment to elide the head.
fn long() -> AppState {
    many(30)
}

/// A file of `count` lines with one of them changed.
fn many(count: usize) -> AppState {
    let mut lines: Vec<String> = (1..=count).map(|at| "ab".repeat(at)).collect();
    let before = lines.join("\n") + "\n";
    lines[19] = "changed".into();
    let after = lines.join("\n") + "\n";
    let mut app = with_files();
    shows(&mut app, "src/lib.rs", &before, &after);
    app
}
