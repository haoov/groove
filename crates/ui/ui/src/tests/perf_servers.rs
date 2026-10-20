//! What the language servers' marks, popup and list cost a frame. Run with
//! `cargo test --release -p groove-ui perf_servers -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::DiffView;

use super::perf::big;
use crate::tests::window;
use crate::{Ui, view};

/// `big(lines)` with a diagnostic on every `every`th line, spread over two servers.
fn diagnosed(lines: usize, every: usize) -> AppState {
    use groove_controllers::workspace_service::ServerKey;
    use groove_types::{Caret, Diagnostic, Severity};
    let mut app = big(lines);
    let worktree = app.workspace.worktree.clone().expect("worktree");
    for name in ["ty", "ruff"] {
        let key = ServerKey {
            worktree: worktree.clone(),
            dir: "/w".into(),
            name: name.into(),
        };
        let found = (0..lines)
            .step_by(every)
            .map(|line| Diagnostic {
                start: Caret::new(line, 3),
                end: Caret::new(line, 12),
                severity: Severity::Warning,
                message: format!("unused value on line {line}"),
                source: Some(name.into()),
                code: None,
            })
            .collect();
        let at = std::path::Path::new("/w/src/lib.rs");
        app.workspace.servers.diagnosed(&key, at, found);
    }
    app
}

#[test]
#[ignore]
fn time_a_frame_with_diagnostics() {
    let mut fonts = Fonts::embedded();
    for (lines, every) in [(2000, 10), (20000, 10), (20000, 1)] {
        for face in [
            crate::views::session::Face::File,
            crate::views::session::Face::Stream(DiffView::Inline),
            crate::views::session::Face::Stream(DiffView::Split),
        ] {
            let mut ui = Ui::default();
            crate::tests::set_face(&mut ui, face);
            let mut each = Vec::new();
            for app in [big(lines), diagnosed(lines, every)] {
                let _ = view(&app, &ui, window(), &mut fonts);
                let runs = 20;
                let started = Instant::now();
                for _ in 0..runs {
                    let _ = view(&app, &ui, window(), &mut fonts);
                }
                each.push(started.elapsed() / runs);
            }
            let found = 2 * lines.div_ceil(every);
            println!(
                "{lines:>6} lines, {found:>6} diagnostics {face:?}: {:>10?} bare, {:>10?} diagnosed",
                each[0], each[1]
            );
        }
    }
}

/// One frame's cost, the mean of twenty after a first.
fn timed(app: &AppState, ui: &Ui, fonts: &mut Fonts) -> std::time::Duration {
    let _ = view(app, ui, window(), fonts);
    let started = Instant::now();
    for _ in 0..20 {
        let _ = view(app, ui, window(), fonts);
    }
    started.elapsed() / 20
}

#[test]
#[ignore]
fn time_the_popup_and_the_completion_list() {
    use groove_types::{Caret, Completion, Edit, Motion};
    let mut fonts = Fonts::embedded();
    let mut app = diagnosed(2000, 10);
    let path = "src/lib.rs";
    let at = Caret::new(0, 5);
    app.workspace.edit(&Edit::Move(Motion::To(at)));
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    crate::tests::set_face(&mut ui, crate::views::session::Face::File);
    let bare = timed(&app, &ui, &mut fonts);
    let code = "def f(\n    a: int,\n    b: dict[str, list[tuple[int, str, bytes]]],\n) -> None\n";
    let markdown = format!("```python\n{code}```\n---\n{}", "Some prose. ".repeat(400));
    app.workspace.servers.asked(path, at, true);
    app.workspace.servers.answered(path, at, Some(markdown));
    let popup = timed(&app, &ui, &mut fonts);
    app.workspace.servers.hovered = None;
    let item = |at: usize| Completion {
        label: format!("name_{at}"),
        detail: Some(format!("def name_{at}(value: usize) -> usize")),
        insert: format!("name_{at}"),
        range: None,
        filter: format!("name_{at}"),
        sort: format!("{at:05}"),
    };
    let asked = app.workspace.servers.offering(path, at);
    app.workspace
        .servers
        .offers(asked, (0..2000).map(item).collect());
    let listed =
        crate::views::session::diff::completion::shown(&app, &ui).map(|one| one.items.len());
    assert_eq!(listed, Some(2000), "the list stands");
    let list = timed(&app, &ui, &mut fonts);
    println!("file view: {bare:?} bare, {popup:?} with the popup, {list:?} with 2000 completions");
}
