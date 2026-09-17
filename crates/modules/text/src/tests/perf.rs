//! What a keystroke costs the buffer. Run with
//! `cargo test --release -p groove-text perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_types::{Caret, Edit, Motion};

use crate::{Buffer, Document};

const RUNS: u32 = 200;

fn buffer(lines: usize) -> Buffer {
    let text: String = (0..lines)
        .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
        .collect();
    Buffer::new(Document::new("src/lib.rs", &text))
}

#[test]
#[ignore]
fn time_a_keystroke() {
    for lines in [300, 3000, 30_000] {
        let mut buffer = buffer(lines);
        buffer.edit(&Edit::Move(Motion::To(Caret::new(lines / 2, 0))));
        let started = Instant::now();
        for _ in 0..RUNS {
            buffer.edit(&Edit::Insert("x".into()));
        }
        let typing = started.elapsed() / RUNS;
        let started = Instant::now();
        for _ in 0..RUNS {
            buffer.edit(&Edit::Undo);
        }
        let undo = started.elapsed() / RUNS;
        println!("{lines:>6} lines: {typing:?} a keystroke, {undo:?} an undo");
    }
}

#[test]
#[ignore]
fn time_reading_the_colours_again() {
    for lines in [300, 3000] {
        let mut buffer = buffer(lines);
        let revision = buffer.revision();
        let started = Instant::now();
        buffer.recolour(revision);
        println!(
            "{lines:>6} lines: {:?} to read the colours",
            started.elapsed()
        );
    }
}
