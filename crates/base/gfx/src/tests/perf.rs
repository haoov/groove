//! What a frame costs the GPU. Run with
//! `cargo test --release -p groove-gfx perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use crate::{Cell, CellGrid, Color, Fonts, Frame, Palette, Renderer, Size};

const RUNS: u32 = 20;

/// A terminal pane the size an agent runs in.
fn grid(cols: usize, rows: usize, at: usize) -> CellGrid {
    let mut grid = CellGrid::new(0.0, 0.0, cols, rows, 13.0);
    let text = "the agent said something about a file and then said it again";
    for row in 0..rows {
        for col in 0..cols {
            let ch = text.as_bytes()[(row * cols + col + at) % text.len()] as char;
            grid.set(col, row, cell(ch));
        }
    }
    grid
}

fn cell(ch: char) -> Cell {
    Cell {
        ch,
        fg: Palette::MOCHA.text,
        bg: Color::TRANSPARENT,
        bold: false,
    }
}

#[test]
#[ignore]
fn time_a_terminal_grid() {
    let size = Size::new(1280, 800);
    let mut renderer = Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter");
    for (cols, rows) in [(80, 24), (120, 40)] {
        let steady = || {
            let mut frame = Frame::new(size, Palette::MOCHA.base);
            frame.grid(grid(cols, rows, 0));
            frame
        };
        renderer.render(&steady()).expect("the first frame");
        let started = Instant::now();
        for at in 0..RUNS {
            let mut frame = Frame::new(size, Palette::MOCHA.base);
            frame.grid(grid(cols, rows, at as usize));
            renderer.render(&frame).expect("a frame of output");
        }
        let moving = started.elapsed() / RUNS;
        let started = Instant::now();
        for _ in 0..RUNS {
            renderer.render(&steady()).expect("a still frame");
        }
        let still = started.elapsed() / RUNS;
        let cells = cols * rows;
        println!("{cols}x{rows} ({cells} cells): {moving:?} a frame moving, {still:?} still");
    }
}
