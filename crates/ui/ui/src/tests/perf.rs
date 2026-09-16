//! What a frame costs. Run with
//! `cargo test --release -p groove-ui perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_controllers::AppState;
use groove_controllers::workspace_service::from_text;
use groove_gfx::{Fonts, Renderer, Size};
use groove_types::DiffView;

use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

fn big(lines: usize) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| w.id.clone())
        .expect("worktree");
    let file = groove_types::FileDiff {
        path: "src/lib.rs".into(),
        added: 1,
        deleted: 1,
        status: groove_types::FileStatus::Modified,
        staged: Some(false),
    };
    app.workspace.loaded(worktree, vec![file]);
    let before: String = (0..lines)
        .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
        .collect();
    let after = before.replace("value + 20 }", "value * 20 }");
    app.workspace.opened = Some(from_text("src/lib.rs", &before, &after));
    app
}

#[test]
#[ignore]
fn time_the_frame() {
    let mut fonts = Fonts::embedded();
    for lines in [200, 2000, 20000] {
        let app = big(lines);
        for view_kind in [DiffView::File, DiffView::Inline, DiffView::Split] {
            let mut ui = Ui::default();
            ui.session.tab = Tab::Diff;
            ui.session.view = view_kind;
            let _ = view(&app, &ui, window(), &mut fonts);
            let started = Instant::now();
            let runs = 20;
            let mut texts = 0;
            for _ in 0..runs {
                let (frame, _) = view(&app, &ui, window(), &mut fonts);
                texts = frame.layers().iter().map(|l| l.texts.len()).sum();
            }
            let each = started.elapsed() / runs;
            println!("{lines:>6} lines {view_kind:?}: {each:?} for {texts} runs");
        }
    }
}

#[test]
#[ignore]
fn time_the_draw() {
    let size = Size::new(crate::tests::WINDOW.0, crate::tests::WINDOW.1);
    let mut renderer = Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter");
    for lines in [300, 3000] {
        let app = big(lines);
        for view_kind in [DiffView::File, DiffView::Inline] {
            let mut ui = Ui::default();
            ui.session.tab = Tab::Diff;
            ui.session.view = view_kind;
            for _ in 0..3 {
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            let runs = 20;
            let started = Instant::now();
            for at in 0..runs {
                ui.session.diff = at as f32;
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            println!(
                "{lines:>5} lines {view_kind:?}: {:?} a frame",
                started.elapsed() / runs
            );
        }
    }
}
