//! What a frame costs. Run with
//! `cargo test --release -p groove-ui perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_controllers::AppState;
use groove_gfx::{Color, Fonts, Renderer, Size};
use groove_types::{DiffView, Rgb, Screen, ScreenCell};

use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

fn big(lines: usize) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("worktree");
    let file = groove_types::FileDiff {
        path: "src/lib.rs".into(),
        added: 1,
        deleted: 1,
        status: groove_types::FileStatus::Modified,
        staged: Some(false),
    };
    app.workspace
        .loaded(worktree, vec![file], Default::default());
    let before: String = (0..lines)
        .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
        .collect();
    let after = before.replace("value + 20 }", "value * 20 }");
    crate::tests::shows(&mut app, "src/lib.rs", &before, &after);
    app
}

/// The same Go file twice: indented with tabs, which the rows run out to their
/// stops, and with those tabs already spaces.
fn tabbed(lines: usize, tabs: bool) -> AppState {
    let mut app = big(lines);
    let indent = match tabs {
        true => "\t\t",
        false => "        ",
    };
    let before: String = (0..lines)
        .map(|at| format!("func name{at}() int {{\n{indent}return {at}\n}}\n"))
        .collect();
    let after = before.replace("return 20", "return 21");
    crate::tests::shows(&mut app, "main.go", &before, &after);
    app
}

#[test]
#[ignore]
fn time_what_tabs_cost() {
    let mut fonts = Fonts::embedded();
    for lines in [200, 2000] {
        for tabs in [false, true] {
            let app = tabbed(lines, tabs);
            let mut ui = Ui::default();
            ui.session.tab = Tab::File;
            ui.session.view = DiffView::Editor;
            let _ = view(&app, &ui, window(), &mut fonts);
            let runs = 50;
            let started = Instant::now();
            for _ in 0..runs {
                let _ = view(&app, &ui, window(), &mut fonts);
            }
            let each = started.elapsed() / runs;
            let kind = match tabs {
                true => "tabs  ",
                false => "spaces",
            };
            println!("{:>6} lines of {kind}: {each:?} a frame", lines * 3);
        }
    }
}

#[test]
#[ignore]
fn time_the_frame() {
    let mut fonts = Fonts::embedded();
    for lines in [200, 2000, 20000] {
        let app = big(lines);
        for view_kind in [DiffView::Editor, DiffView::Inline, DiffView::Split] {
            let mut ui = Ui::default();
            ui.session.tab = Tab::File;
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
        for view_kind in [DiffView::Editor, DiffView::Inline] {
            let mut ui = Ui::default();
            ui.session.tab = Tab::File;
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

/// A pane of agent output, as the terminal hands it over.
fn screen(cols: usize, rows: usize) -> Screen {
    let text = "the agent said something about a file and then said it again";
    let cells = (0..cols * rows)
        .map(|at| ScreenCell {
            ch: text.as_bytes()[at % text.len()] as char,
            fg: Rgb {
                r: 205,
                g: 214,
                b: 244,
            },
            bg: None,
            bold: false,
            spacer: false,
        })
        .collect();
    Screen {
        cols,
        rows,
        cells,
        cursor: Some((0, 0)),
        selected: Vec::new(),
    }
}

#[test]
#[ignore]
fn time_the_agent_pane() {
    for (cols, rows) in [(80, 24), (120, 40)] {
        let screen = screen(cols, rows);
        let runs = 100;
        let started = Instant::now();
        for _ in 0..runs {
            let grid = crate::widget::grid_of(&screen, 0.0, 0.0, 13.0, Color::TRANSPARENT);
            assert_eq!(grid.cells.len(), cols * rows);
        }
        println!(
            "{cols}x{rows}: {:?} to turn a screen into a grid",
            started.elapsed() / runs
        );
    }
}

/// `count` changed files, each a handful of rows, all on screen at once.
fn several(count: usize) -> AppState {
    use groove_controllers::workspace_service::{Document, Painted, from_text};
    let mut app = full_app();
    let sides: Vec<(String, String, String)> = (0..count)
        .map(|at| {
            let before: String = (0..60)
                .map(|n| format!("fn name_{at}_{n}(value: usize) -> usize {{ value + {n} }}\n"))
                .collect();
            let after = before.replace("value + 4 }", "value * 4 }");
            (format!("src/file_{at}.rs"), before, after)
        })
        .collect();
    let named: Vec<(&str, &str, &str)> = sides
        .iter()
        .map(|(path, before, after)| (path.as_str(), before.as_str(), after.as_str()))
        .collect();
    crate::tests::changed_files(&mut app, &named);
    for (path, before, after) in &sides {
        app.workspace.coloured.insert(
            path.clone(),
            Painted {
                old: Document::new(path, before),
                new: Document::new(path, after),
            },
        );
    }
    let (path, before, after) = &sides[0];
    app.workspace.opened = Some(from_text(path, before, after));
    app
}

#[test]
#[ignore]
fn time_a_frame_over_several_files() {
    use groove_types::{Caret, Edit, Motion};
    let size = Size::new(crate::tests::WINDOW.0, crate::tests::WINDOW.1);
    let mut renderer = Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter");
    for files in [1, 5, 40] {
        for kind in [DiffView::Inline, DiffView::Split] {
            let mut app = several(files);
            let mut ui = Ui::default();
            ui.session.tab = Tab::File;
            ui.session.view = kind;
            ui.focus = crate::Focus::Workspace;
            let (frame, _) = view(&app, &ui, window(), renderer.fonts());
            renderer.render(&frame).expect("rendered");
            let runs = 20;
            let started = Instant::now();
            for _ in 0..runs {
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            let still = started.elapsed() / runs;

            let buffer = &mut app.workspace.opened.as_mut().unwrap().new;
            buffer.edit(&Edit::Move(Motion::To(Caret::new(4, 8))));
            let started = Instant::now();
            for _ in 0..runs {
                app.workspace
                    .opened
                    .as_mut()
                    .unwrap()
                    .new
                    .edit(&Edit::Insert("x".into()));
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            let typing = started.elapsed() / runs;
            let started = Instant::now();
            for at in 0..runs {
                ui.session.diff = at as f32 * crate::tokens::Tokens::new(1.0).line;
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            println!(
                "{files} files {kind:?}: {still:>10?} still, {typing:>10?} typing, {:>10?} scrolling",
                started.elapsed() / runs
            );
        }
    }
}
