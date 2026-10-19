//! What the editor's outline costs: guides, folds and the word's occurrences, still and
//! while typing. Run with `cargo test --release -p groove-ui perf_editor -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::{Duration, Instant};

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{Caret, Edit, Motion};

use crate::tests::{full_app, shows, window};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

/// `classes` classes of a method each, six lines a class.
fn python(classes: usize) -> AppState {
    let body: String = (0..classes)
        .map(|at| {
            format!("class C{at}:\n    def f(self, x):\n        if x:\n            return x\n        return {at}\n\n")
        })
        .collect();
    let mut app = full_app();
    shows(&mut app, "src/a.py", &body, &body);
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

/// One frame after the parse job landed, the mean of `runs`, each after `edit`, which is not timed.
fn framed(app: &mut AppState, ui: &mut Ui, runs: u32, edit: Option<&Edit>) -> Duration {
    let mut fonts = Fonts::embedded();
    ui.settle(app);
    let _ = view(app, ui, window(), &mut fonts);
    let mut spent = Duration::ZERO;
    for _ in 0..runs {
        if let Some(edit) = edit {
            app.workspace.edit(edit);
            parsed(app);
        }
        let started = Instant::now();
        ui.settle(app);
        let _ = view(app, ui, window(), &mut fonts);
        spent += started.elapsed();
    }
    spent / runs
}

/// What the derive job hands back, landed on the open buffer.
fn parsed(app: &mut AppState) {
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    let doc = app.workspace.active().expect("open").new.document().clone();
    let read = groove_controllers::workspace_service::derived("src/a.py", &doc, doc.clone());
    let revision = app.workspace.active().expect("open").new.revision();
    app.workspace
        .derived((&worktree, "src/a.py"), read, revision);
}

#[test]
#[ignore]
fn time_the_editor_outline_still_and_while_typing() {
    for classes in [300, 2000] {
        let mut app = python(classes);
        let mut ui = on_file();
        let middle = Caret::new(classes * 3, 8);
        app.workspace.edit(&Edit::Move(Motion::To(middle)));
        let still = framed(&mut app, &mut ui, 20, None);
        let bare = framed(&mut app, &mut ui, 20, Some(&Edit::Insert("y".into())));
        ui.session.word = Some("x".into());
        let worded = framed(&mut app, &mut ui, 20, None);
        let folds = ui.session.folds.entry("src/a.py".into()).or_default();
        folds
            .starts
            .extend((0..classes).step_by(2).map(|at| at * 6));
        let folded = framed(&mut app, &mut ui, 20, None);
        let typed = framed(&mut app, &mut ui, 20, Some(&Edit::Insert("y".into())));
        let broken = framed(&mut app, &mut ui, 20, Some(&Edit::Newline));
        println!(
            "{:>6} lines: {still:>10?} still, {bare:>10?} a keystroke unfolded, {worded:>10?} with the word, {folded:>10?} with {} folds, \
             {typed:>10?} a keystroke, {broken:>10?} a new line",
            classes * 6,
            classes / 2,
        );
    }
}
