use std::sync::mpsc;
use std::time::Duration;

use groove_types::{AnsiPalette, Rgb};

use crate::{Hooks, PtySpec, Terminal};

const P: AnsiPalette = AnsiPalette::MOCHA;

/// `sh -c script` on a terminal; the receiver gets the exit code.
fn run(script: &str, cols: u16, rows: u16) -> (Terminal, mpsc::Receiver<u32>) {
    let (tx, rx) = mpsc::channel();
    let spec = PtySpec {
        program: "sh".into(),
        args: vec!["-c".into(), script.into()],
        cwd: "/".into(),
        env: vec![],
        rows,
        cols,
    };
    let hooks = Hooks {
        on_damage: Box::new(|| {}),
        on_exit: Box::new(move |code| {
            let _ = tx.send(code);
        }),
    };
    (Terminal::spawn(spec, P, hooks).unwrap(), rx)
}

fn wait(rx: &mpsc::Receiver<u32>) -> u32 {
    rx.recv_timeout(Duration::from_secs(10))
        .expect("the child exits")
}

#[test]
fn text_lands_on_the_grid_and_the_exit_code_comes_last() {
    let (term, rx) = run("printf 'hello\\nworld'; exit 3", 20, 5);
    assert_eq!(wait(&rx), 3);
    let screen = term.screen();
    assert_eq!((screen.cols, screen.rows), (20, 5));
    assert_eq!(screen.line(0), "hello");
    assert_eq!(screen.line(1), "world");
    assert_eq!(screen.cursor, Some((5, 1)));
}

#[test]
fn ansi_colours_resolve_through_the_palette() {
    let (term, rx) = run(
        "printf '\\033[31mr\\033[0m\\033[1;32mg\\033[0m\\033[44mb\\033[0m\\033[7mi\\033[0m\\033[38;5;196mx\\033[38;2;1;2;3my'",
        20,
        2,
    );
    wait(&rx);
    let s = term.screen();
    assert_eq!(s.cell(0, 0).fg, P.colors[1]);
    assert!(s.cell(1, 0).bold);
    assert_eq!(s.cell(1, 0).fg, P.colors[2]);
    assert_eq!(s.cell(2, 0).bg, Some(P.colors[4]));
    assert_eq!(
        (s.cell(3, 0).fg, s.cell(3, 0).bg),
        (P.background, Some(P.foreground))
    );
    assert_eq!(s.cell(4, 0).fg, Rgb { r: 255, g: 0, b: 0 });
    assert_eq!(s.cell(5, 0).fg, Rgb { r: 1, g: 2, b: 3 });
    assert_eq!(
        s.cell(6, 0).bg,
        None,
        "the default background stays transparent"
    );
}

#[test]
fn a_wide_character_takes_two_cells() {
    let (term, rx) = run("printf '日本x'", 20, 2);
    wait(&rx);
    let s = term.screen();
    assert_eq!(s.cell(0, 0).ch, '日');
    assert!(s.cell(1, 0).spacer);
    assert_eq!(s.cell(2, 0).ch, '本');
    assert_eq!(s.cell(4, 0).ch, 'x');
    assert_eq!(s.line(0), "日本x");
}

#[test]
fn a_resize_reaches_the_grid_and_the_child() {
    let (term, rx) = run("sleep 0.4; stty size", 40, 10);
    term.resize(30, 8).unwrap();
    assert_eq!(term.size(), (30, 8));
    wait(&rx);
    let s = term.screen();
    assert_eq!((s.cols, s.rows), (30, 8));
    assert_eq!(s.line(0), "8 30");
}

#[test]
fn input_reaches_the_child() {
    let (term, rx) = run("read line; printf \"got %s\" \"$line\"", 20, 3);
    term.write(b"abc\n").unwrap();
    wait(&rx);
    let s = term.screen();
    assert_eq!(s.line(0), "abc", "the tty echoes the input");
    assert_eq!(s.line(1), "got abc");
}

#[test]
fn terminate_ends_a_waiting_child() {
    let (term, rx) = run("sleep 30", 10, 2);
    term.terminate().unwrap();
    let code = wait(&rx);
    assert_ne!(code, 0);
}
