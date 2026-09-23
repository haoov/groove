//! What reading a file costs. Run with
//! `cargo test --release -p groove-diff perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::path::Path;
use std::time::Instant;

use crate::tests::summary::{git, repo, write};
use crate::{Opened, opened, reopened};

const RUNS: u32 = 10;

fn source(lines: usize) -> String {
    (0..lines)
        .map(|at| format!("fn name_{at}(value: usize) -> usize {{ value + {at} }}\n"))
        .collect()
}

fn read(dir: &Path, path: &str) -> Opened {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime
        .block_on(opened(dir, path, "HEAD"))
        .expect("the file opens")
}

#[test]
#[ignore]
fn time_reading_a_file() {
    for lines in [300, 3000] {
        let dir = repo();
        let (path, text) = ("src/big.rs", source(lines));
        write(dir.path(), path, &text);
        git(dir.path(), &["add", path]);
        git(dir.path(), &["commit", "-m", "big"]);
        write(
            dir.path(),
            path,
            &text.replace("value + 20 }", "value * 20 }"),
        );

        let started = Instant::now();
        let mut file = read(dir.path(), path);
        for _ in 1..RUNS {
            file = read(dir.path(), path);
        }
        let full = started.elapsed() / RUNS;

        let started = Instant::now();
        for _ in 0..RUNS {
            reopened(dir.path(), path, file.old.clone());
        }
        let again = started.elapsed() / RUNS;
        println!("{lines:>5} lines: open {full:?}, reopen {again:?}");
    }
}

/// The word diff's own worst case: every line of the file changed, one word each.
#[test]
#[ignore]
fn time_aligning_a_file_changed_line_by_line() {
    for lines in [300, 3000] {
        let before = source(lines);
        let after = before.replace("value +", "value *");
        let started = Instant::now();
        for _ in 0..RUNS {
            crate::aligned("src/big.rs", &before, &after);
        }
        let whole = started.elapsed() / RUNS;
        let (old, new) = (
            crate::Document::plain("src/big.rs", &before),
            crate::Document::plain("src/big.rs", &after),
        );
        let rows = crate::align(&old, &new, crate::alignment::CONTEXT);
        let started = Instant::now();
        for _ in 0..RUNS {
            crate::alignment::words(&rows, &old, &new);
        }
        println!(
            "{lines:>5} lines: align {whole:?}, of it words {:?}",
            started.elapsed() / RUNS
        );
    }
}
