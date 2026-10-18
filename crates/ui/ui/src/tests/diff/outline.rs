//! Scope guides, the word at the caret's occurrences, and the blocks a click folds.

use groove_controllers::AppState;
use groove_gfx::{Color, Fonts, Quad};
use groove_types::{Caret, Edit, Motion};
use groove_ui_kit::base::style::Styles;
use groove_ui_kit::base::tokens::{Tokens, WORD_MS};

use super::{row_texts, with_files};
use crate::hit::Target;
use crate::tests::{click, shows, window};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

const SOURCE: &str = "class A:\n    def f(self):\n        x = 1\n        return x\n\nb = A()\n";

fn python() -> AppState {
    let mut app = with_files();
    shows(&mut app, "src/a.py", SOURCE, SOURCE);
    app
}

fn on_file() -> Ui {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    ui.session.tab = Tab::Files;
    ui
}

/// The quads of the rows drawn in `color`.
fn painted(app: &AppState, ui: &Ui, color: impl Fn(&Styles) -> Color) -> Vec<Quad> {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let color = color(&styles);
    frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == color && code.contains(quad.rect.x, quad.rect.y))
        .copied()
        .collect()
}

#[test]
fn a_guide_runs_down_each_scope_at_the_indent_of_its_opening_line() {
    let guides = painted(&python(), &on_file(), Styles::guide);
    let mut columns: Vec<u32> = guides.iter().map(|quad| quad.rect.x as u32).collect();
    columns.sort();
    columns.dedup();
    assert_eq!(
        guides.len(),
        5,
        "the class's three lines, the method's two: {guides:?}"
    );
    assert_eq!(columns.len(), 2, "one column a scope");
}

#[test]
fn a_click_on_a_block_s_caret_folds_it_under_its_line_and_says_how_much_it_hides() {
    let app = python();
    let mut ui = on_file();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let caret = hits
        .rect_of(&Target::Block(0))
        .expect("the class opens a block");
    assert!(
        hits.rect_of(&Target::Block(4)).is_none(),
        "a blank line opens none"
    );
    click(caret, &mut ui, &app, &hits);
    let shown = row_texts(&app, &ui);
    assert!(!shown.iter().any(|one| one.contains("return")), "{shown:?}");
    assert!(shown.iter().any(|one| one == "⋯ 3 lines"), "{shown:?}");
    assert!(
        shown.iter().any(|one| one.contains('b')),
        "the line after it stays"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let caret = hits
        .rect_of(&Target::Block(0))
        .expect("it folds open again");
    click(caret, &mut ui, &app, &hits);
    assert!(
        row_texts(&app, &ui)
            .iter()
            .any(|one| one.contains("return"))
    );
}

#[test]
fn a_fold_the_caret_stands_inside_stays_open() {
    let mut app = python();
    let mut ui = on_file();
    ui.session
        .folds
        .entry("src/a.py".into())
        .or_default()
        .starts
        .insert(0);
    assert!(
        !row_texts(&app, &ui)
            .iter()
            .any(|one| one.contains("return"))
    );
    app.workspace
        .edit(&Edit::Move(Motion::To(Caret::new(3, 9))));
    assert!(
        row_texts(&app, &ui)
            .iter()
            .any(|one| one.contains("return"))
    );
}

#[test]
fn the_word_the_caret_rests_in_is_grounded_wherever_it_stands() {
    let mut app = python();
    let mut ui = on_file();
    app.workspace
        .edit(&Edit::Move(Motion::To(Caret::new(2, 8))));
    crate::input::word(&mut ui, &app, 1000);
    assert_eq!(ui.session.word, None, "not before it rests");
    crate::input::word(&mut ui, &app, 1000 + WORD_MS);
    assert_eq!(ui.session.word.as_deref(), Some("x"));
    let grounded = painted(&app, &ui, Styles::occurs);
    assert_eq!(
        grounded.len(),
        2,
        "`x = 1` and `return x`, not the x of `self`"
    );
    app.workspace
        .edit(&Edit::Move(Motion::To(Caret::new(5, 0))));
    crate::input::word(&mut ui, &app, 2000);
    assert_eq!(ui.session.word, None, "a move lets the word go");
}

#[test]
fn a_fold_follows_its_block_when_lines_come_or_go_above_it() {
    let mut app = python();
    let mut ui = on_file();
    ui.session
        .folds
        .entry("src/a.py".into())
        .or_default()
        .starts
        .insert(1);
    app.workspace
        .edit(&Edit::Move(Motion::To(Caret::new(0, 0))));
    app.workspace.edit(&Edit::Newline);
    ui.settle(&app);
    let starts = |ui: &Ui| ui.session.folds["src/a.py"].starts.clone();
    assert_eq!(starts(&ui), [2].into(), "the method moved down a line");
    assert!(
        !row_texts(&app, &ui)
            .iter()
            .any(|one| one.contains("return"))
    );
    app.workspace.edit(&Edit::Backspace);
    ui.settle(&app);
    assert_eq!(starts(&ui), [1].into(), "and back up");
}

#[test]
fn the_word_at_the_caret_is_grounded_in_the_diff_too() {
    let mut app = with_files();
    shows(
        &mut app,
        "src/a.py",
        &SOURCE.replace("x = 1", "x = 0"),
        SOURCE,
    );
    let mut ui = on_file();
    ui.session.tab = Tab::Diff;
    ui.session.word = Some("x".into());
    app.workspace
        .edit(&Edit::Move(Motion::To(Caret::new(2, 8))));
    let grounded = painted(&app, &ui, Styles::occurs);
    assert_eq!(
        grounded.len(),
        3,
        "the line removed, the line added, the return"
    );
}
