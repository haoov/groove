//! What the frames after a move cost, drawn and rendered: the palette opening, a resize, a session seen first.
//! Run with
//! `cargo test --release -p groove-ui perf_moves -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::{Duration, Instant};

use groove_controllers::AppState;
use groove_gfx::{Fonts, Renderer, Size};

use super::chat::{chat_ui, conversing};
use super::perf_chat::chatty;
use crate::input::Key;
use crate::tests::{ALT_SHIFT, WINDOW, metrics, press};
use crate::{Ui, view};

/// One frame drawn and rendered at `width`.
fn frame(app: &AppState, ui: &Ui, renderer: &mut Renderer, width: u32) -> Duration {
    let started = Instant::now();
    let (frame, _) = view(app, ui, metrics(width, WINDOW.1, 1.0), renderer.fonts());
    renderer.render(&frame).expect("rendered");
    started.elapsed()
}

fn steady(app: &AppState, ui: &Ui, renderer: &mut Renderer) -> Duration {
    (0..3).for_each(|_| _ = frame(app, ui, renderer, WINDOW.0));
    let runs = 20;
    (0..runs)
        .map(|_| frame(app, ui, renderer, WINDOW.0))
        .sum::<Duration>()
        / runs
}

fn renderer() -> Renderer {
    let size = Size::new(WINDOW.0, WINDOW.1);
    Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter")
}

#[test]
#[ignore]
fn time_the_frames_after_a_move() {
    for conversation in [false, true] {
        let mut app = chatty(200);
        conversing(&mut app, conversation);
        let mut renderer = renderer();
        let cold = frame(&app, &chat_ui(), &mut renderer, WINDOW.0);
        let mut ui = chat_ui();
        let still = steady(&app, &ui, &mut renderer);

        press(Key::Char(';'), ALT_SHIFT, &mut ui, &app);
        assert!(ui.palette().is_some(), "the palette opened");
        let opened = frame(&app, &ui, &mut renderer, WINDOW.0);
        let open = steady(&app, &ui, &mut renderer);
        press(Key::Char('s'), Default::default(), &mut ui, &app);
        let typed = frame(&app, &ui, &mut renderer, WINDOW.0);

        let ui = chat_ui();
        let runs = 30;
        let resized: Duration = (0..runs)
            .map(|at| frame(&app, &ui, &mut renderer, WINDOW.0 - at * 7))
            .sum::<Duration>()
            / runs;
        println!(
            "conversation {conversation:>5}: {cold:>10?} cold, {still:>10?} still, \
             {opened:>10?} palette opened, {open:>10?} palette open, {typed:>10?} typed, \
             {resized:>10?} a resize"
        );
    }
}
