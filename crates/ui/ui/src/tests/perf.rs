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

#[test]
#[ignore]
fn time_the_frame() {
    let mut fonts = Fonts::embedded();
    for lines in [200, 2000, 20000] {
        let app = big(lines);
        for view_kind in [
            crate::views::session::Face::File,
            crate::views::session::Face::Stream(DiffView::Inline),
            crate::views::session::Face::Stream(DiffView::Split),
        ] {
            let mut ui = Ui::default();
            crate::tests::set_face(&mut ui, view_kind);
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

/// `big(lines)` with a long note, answered once, on every tenth line.
fn noted(lines: usize) -> AppState {
    use groove_types::{Anchor, Note, NoteOrigin, Said, Timestamp};
    let said = |author: &str, body: &str| Said {
        author: author.into(),
        body: body.into(),
        at: Timestamp::new(0),
    };
    let body = "issue (blocking): this leaks the handle every time the file is opened \
                again, and nothing ever closes it.\n\n- close it on drop\n- or keep one \
                handle for the whole session and hand out `&File` to the callers";
    let mut app = big(lines);
    app.delivery.shown = (0..lines as u32)
        .step_by(10)
        .map(|line| Note {
            origin: NoteOrigin::Thread(format!("t{line}")),
            anchor: Some(Anchor::line("src/lib.rs", line + 1)),
            resolved: false,
            said: vec![
                said("reviewer", body),
                said("haoov", "fixed, it closes on drop now"),
            ],
        })
        .collect();
    app
}

/// One frame at `width`: the notes wrap to it in that frame.
fn resize(app: &AppState, ui: &Ui, width: u32, fonts: &mut Fonts) -> usize {
    let window = crate::tests::metrics(width, crate::tests::WINDOW.1, 1.0);
    view(app, ui, window, fonts).1.wrap()
}

#[test]
#[ignore]
fn time_a_frame_with_notes() {
    let mut fonts = Fonts::embedded();
    let (wide, narrow) = (crate::tests::WINDOW.0 * 3 / 2, crate::tests::WINDOW.0);
    for lines in [200, 2000] {
        let app = noted(lines);
        for face in [
            crate::views::session::Face::File,
            crate::views::session::Face::Stream(DiffView::Inline),
        ] {
            let mut ui = Ui::default();
            crate::tests::set_face(&mut ui, face);
            let cols = resize(&app, &ui, narrow, &mut fonts);
            let rows = crate::views::session::diff::rows_of;
            assert!(
                rows(&app, &ui, cols) > rows(&big(lines), &ui, cols),
                "{face:?} draws the notes"
            );
            let runs = 20;
            let started = Instant::now();
            for _ in 0..runs {
                let _ = view(&app, &ui, window(), &mut fonts);
            }
            let still = started.elapsed() / runs;
            let started = Instant::now();
            for at in 0..runs {
                let width = if at % 2 == 0 { wide } else { narrow };
                resize(&app, &ui, width, &mut fonts);
            }
            let resized = started.elapsed() / runs;
            println!("{lines:>5} lines {face:?}: {still:>10?} still, {resized:>10?} a resize");
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
        for view_kind in [
            crate::views::session::Face::File,
            crate::views::session::Face::Stream(DiffView::Inline),
        ] {
            let mut ui = Ui::default();
            crate::tests::set_face(&mut ui, view_kind);
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
            let grid = groove_ui_kit::widgets::grid_of(&screen, 0.0, 0.0, 13.0, Color::TRANSPARENT);
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
    crate::tests::open_file(&mut app, from_text(path, before, after));
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
            ui.session.tab = Tab::Diff;
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

            let buffer = &mut app.workspace.active_mut().unwrap().new;
            buffer.edit(&Edit::Move(Motion::To(Caret::new(4, 8))));
            let started = Instant::now();
            for _ in 0..runs {
                app.workspace
                    .active_mut()
                    .unwrap()
                    .new
                    .edit(&Edit::Insert("x".into()));
                let (frame, _) = view(&app, &ui, window(), renderer.fonts());
                renderer.render(&frame).expect("rendered");
            }
            let typing = started.elapsed() / runs;
            let started = Instant::now();
            for at in 0..runs {
                ui.session.diff = at as f32 * groove_ui_kit::base::tokens::Tokens::new(1.0).line;
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

#[test]
#[ignore]
fn time_a_frame_with_many_files_open() {
    use groove_types::{Caret, Edit, Motion};
    let mut fonts = Fonts::embedded();
    for open in [1, 10, 50] {
        let mut app = several(open);
        for (path, before, after) in sides_of(&app) {
            let file = groove_controllers::workspace_service::from_text(&path, &before, &after);
            crate::tests::open_file(&mut app, file);
            let buffer = &mut app.workspace.active_mut().unwrap().new;
            buffer.edit(&Edit::Move(Motion::To(Caret::new(4, 8))));
            buffer.edit(&Edit::Insert("x".into()));
        }
        for face in [
            crate::views::session::Face::Stream(DiffView::Inline),
            crate::views::session::Face::File,
        ] {
            let mut ui = Ui::default();
            crate::tests::set_face(&mut ui, face);
            ui.focus = crate::Focus::Workspace;
            let _ = view(&app, &ui, window(), &mut fonts);
            let runs = 20;
            let started = Instant::now();
            for _ in 0..runs {
                let _ = view(&app, &ui, window(), &mut fonts);
            }
            let still = started.elapsed() / runs;
            let started = Instant::now();
            for _ in 0..runs {
                let buffer = &mut app.workspace.active_mut().unwrap().new;
                buffer.edit(&Edit::Insert("x".into()));
                let _ = view(&app, &ui, window(), &mut fonts);
            }
            let typing = started.elapsed() / runs;
            println!("{open:>3} open {face:?}: {still:>10?} still, {typing:>10?} typing");
        }
    }
}

/// Each changed file's path and both of its sides, as the fixture wrote them.
fn sides_of(app: &AppState) -> Vec<(String, String, String)> {
    app.workspace
        .coloured
        .iter()
        .map(|(path, painted)| (path.clone(), painted.old.text(), painted.new.text()))
        .collect()
}

/// `count` changed files over a hundred-odd directories, in a worktree of `tree` files.
fn spread(count: usize, tree: usize) -> AppState {
    let mut app = full_app();
    app.workspace.worktree = app.session.selected_worktree().map(|one| one.id.clone());
    let sides: Vec<(String, String, String)> = (0..count)
        .map(|at| {
            let before: String = (0..40).map(|n| format!("key_{n}: value {at}\n")).collect();
            let after = before.replace("key_3: ", "key_3: changed ");
            (
                format!("charts/c{}/templates/t{at}.yaml", at % 200),
                before,
                after,
            )
        })
        .collect();
    let named: Vec<(&str, &str, &str)> = sides
        .iter()
        .map(|(path, before, after)| (path.as_str(), before.as_str(), after.as_str()))
        .collect();
    crate::tests::changed_files(&mut app, &named);
    let paths = (0..tree)
        .map(|at| groove_types::FileDiff {
            path: format!("charts/c{}/files/f{at}.txt", at % 300),
            added: 0,
            deleted: 0,
            status: groove_types::FileStatus::Unchanged,
            staged: None,
        })
        .collect();
    app.workspace.set_paths(paths);
    let (path, before, after) = &sides[0];
    let file = groove_controllers::workspace_service::from_text(path, before, after);
    crate::tests::open_file(&mut app, file);
    app
}

#[test]
#[ignore]
fn time_a_frame_over_many_changed_files() {
    let mut fonts = Fonts::embedded();
    let app = spread(1354, 12_000);
    for (tab, browse) in [
        (Tab::Files, false),
        (Tab::Files, true),
        (Tab::Diff, false),
        (Tab::Diff, true),
    ] {
        let mut ui = Ui::default();
        ui.session.tab = tab;
        if browse {
            ui.session.bar.path.clear();
        } else {
            ui.session.bar.path.set("t");
        }
        let _ = view(&app, &ui, window(), &mut fonts);
        let runs = 20;
        let started = Instant::now();
        for _ in 0..runs {
            let _ = view(&app, &ui, window(), &mut fonts);
        }
        println!(
            "{tab:?} browse={browse}: {:?} a frame",
            started.elapsed() / runs
        );
    }
}

#[test]
#[ignore]
fn time_the_file_lists_parts() {
    use crate::views::session::files::{explorer, listing, narrowed};
    let app = spread(1354, 12_000);
    let mut ui = Ui::default();
    ui.session.tab = Tab::Files;
    let runs = 20;
    let time = |label: &str, f: &dyn Fn()| {
        let started = Instant::now();
        for _ in 0..runs {
            f();
        }
        println!("{label}: {:?}", started.elapsed() / runs);
    };
    time("narrowed", &|| drop(narrowed(&app, &ui)));
    let files = narrowed(&app, &ui);
    time("listing", &|| drop(listing(&files)));
    time("explorer rows", &|| {
        drop(explorer::rows(
            (app.workspace.paths(), app.workspace.paths_stamp()),
            &files,
            &ui,
        ))
    });
    ui.session.bar.path.set("t1");
    time("narrowed by a query", &|| drop(narrowed(&app, &ui)));
}
