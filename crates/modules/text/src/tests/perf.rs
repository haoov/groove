//! What a keystroke costs the buffer. Run with
//! `cargo test --release -p groove-text perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_types::{Caret, Edit, Motion};

use crate::{Buffer, Document};

const RUNS: u32 = 200;

/// The grammar and its query, compiled before anything is timed.
fn warm() {
    let doc = Document::new("src/lib.rs", "fn one() {}\n");
    assert!(doc.is_highlighted());
}

fn buffer(lines: usize) -> Buffer {
    Buffer::new(document(lines))
}

fn document(lines: usize) -> Document {
    let text: String = (0..lines)
        .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
        .collect();
    Document::new("src/lib.rs", &text)
}

#[test]
#[ignore]
fn time_a_keystroke() {
    warm();
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
fn time_opening_a_file() {
    warm();
    for lines in [300, 3000, 30_000] {
        let text: String = (0..lines)
            .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
            .collect();
        let started = Instant::now();
        let doc = Document::new("src/lib.rs", &text);
        let parsed = started.elapsed();
        let rows = lines / 2..lines / 2 + 40;
        let started = Instant::now();
        let window = doc.colours(rows.clone());
        println!(
            "{lines:>6} lines: {parsed:>12?} to parse, {:>12?} to colour 40 rows, {} spans a row",
            started.elapsed(),
            window.of(rows.start).len()
        );
    }
}

#[test]
#[ignore]
fn time_settling_after_a_keystroke() {
    warm();
    for lines in [300, 3000] {
        for (place, column) in [("a line start", 0), ("mid line", 20)] {
            for typed in ["x", "\""] {
                let mut doc = document(lines);
                let at = doc.char_of(Caret::new(lines / 2, column));
                doc.insert(at, typed);
                let cold = {
                    let text = doc.text();
                    let started = Instant::now();
                    Document::new("src/lib.rs", &text);
                    started.elapsed()
                };
                let started = Instant::now();
                doc.reparse();
                println!(
                    "{lines:>6} lines, {typed:?} at {place:>12}: {:>12?} settled, {cold:>12?} from cold",
                    started.elapsed()
                );
            }
        }
    }
}
