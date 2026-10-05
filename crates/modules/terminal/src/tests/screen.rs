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

/// Until the screen shows `text`, which the child prints once it is ready.
#[track_caller]
fn shown(term: &Terminal, text: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let screen = term.screen();
        let all: String = (0..screen.rows).map(|at| screen.line(at)).collect();
        if all.contains(text) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "`{text}` never showed"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
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
fn a_new_palette_recolours_what_is_already_on_the_grid() {
    let (term, rx) = run("printf '\\033[31mr\\033[0mp'", 20, 2);
    wait(&rx);
    let latte = AnsiPalette::LATTE;
    term.recolor(latte);
    let s = term.screen();
    assert_eq!(s.cell(0, 0).fg, latte.colors[1]);
    assert_eq!(s.cell(1, 0).fg, latte.foreground);
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

#[test]
fn the_wheel_reaches_a_program_that_reads_the_mouse() {
    let read = "printf '\\033[?1000h\\033[?1006hready'; read -r one; printf 'got %s' \"${one#?}\"";
    let (term, rx) = run(read, 40, 4);
    shown(&term, "ready");
    term.wheel(1, (3, 2)).unwrap();
    term.write(b"\n").unwrap();
    wait(&rx);
    let said = term.screen().line(1);
    assert!(
        said.contains("[<64;4;3M"),
        "the wheel, at its own cell: {said}"
    );
}

#[test]
fn the_wheel_moves_a_plain_screen_and_types_nothing() {
    let lines = "for i in 1 2 3 4 5 6 7 8; do echo line$i; done; read -r one";
    let (term, rx) = run(lines, 20, 4);
    shown(&term, "line8");
    let before = term.screen().line(0);
    term.wheel(2, (0, 0)).unwrap();
    let after = term.screen().line(0);
    assert_ne!(before, after, "the lines it holds moved");

    term.write(b"\n").unwrap();
    wait(&rx);
    let said = term.screen().line(3);
    assert!(
        !said.contains("64;"),
        "nothing was typed at the child: {said}"
    );
}

#[test]
fn a_selection_takes_the_cells_it_covers() {
    let (term, rx) = run("echo hello world; sleep 0.2", 20, 3);
    wait(&rx);
    term.select_from((0, 0), crate::Select::Cells);
    term.select_to((4, 0));
    assert_eq!(term.selected().as_deref(), Some("hello"));

    let spans = term.screen().selected;
    assert_eq!(spans.len(), 1);
    assert_eq!((spans[0].row, spans[0].from, spans[0].to), (0, 0, 5));

    term.select_nothing();
    assert!(term.selected().is_none());
    assert!(term.screen().selected.is_empty());
}

#[test]
fn a_word_and_a_line_are_taken_whole() {
    let (term, rx) = run("echo hello world; sleep 0.2", 20, 3);
    wait(&rx);
    term.select_from((7, 0), crate::Select::Word);
    assert_eq!(term.selected().as_deref(), Some("world"));

    term.select_from((2, 0), crate::Select::Line);
    assert_eq!(
        term.selected().as_deref(),
        Some("hello world\n"),
        "a line carries its own end"
    );
}

#[test]
fn what_the_program_repaints_does_not_take_the_selection_away() {
    let paint = "printf '\\033[2J\\033[Hhello world\\n'";
    let script = format!("{paint}; read -r a; {paint}; printf again; read -r b");
    let (term, rx) = run(&script, 20, 3);
    shown(&term, "hello world");
    term.select_from((0, 0), crate::Select::Cells);
    term.write(b"\n").unwrap();
    shown(&term, "again");
    term.select_to((4, 0));
    assert_eq!(
        term.selected().as_deref(),
        Some("hello"),
        "a repaint between the two ends of a drag"
    );
    term.write(b"\n").unwrap();
    wait(&rx);
    assert_eq!(
        term.selected().as_deref(),
        Some("hello"),
        "and a repaint after it"
    );
}

#[test]
fn a_program_that_asks_for_motion_is_sent_the_drag() {
    let ask = "printf '\\033[?1003h\\033[?1006hready'";
    let (term, rx) = run(&format!("{ask}; read -r one"), 20, 4);
    shown(&term, "ready");
    term.click((3, 2), true).unwrap();
    term.drag((6, 2)).unwrap();
    term.click((6, 2), false).unwrap();
    shown(&term, "[<32;7;3M");
    let screen = term.screen();
    let said: String = (0..screen.rows).map(|at| screen.line(at)).collect();
    assert!(
        said.contains("[<32;7;3M"),
        "the drag, at its own cell: {said}"
    );
    term.write(b"\n").unwrap();
    wait(&rx);
}

#[test]
fn a_paste_is_bracketed_only_for_a_program_that_asked() {
    let (plain, rx) = run("printf ready; read -r one; printf 'got %s' \"$one\"", 40, 4);
    shown(&plain, "ready");
    plain.paste("hello\n").unwrap();
    wait(&rx);
    assert!(
        plain.screen().line(1).contains("got hello"),
        "{:?}",
        plain.screen().line(1)
    );

    let (asked, rx) = run("printf '\\033[?2004hready'; read -r one", 40, 4);
    shown(&asked, "ready");
    asked.paste("hello").unwrap();
    shown(&asked, "[200~hello");
    let screen = asked.screen();
    let said: String = (0..screen.rows).map(|at| screen.line(at)).collect();
    assert!(
        said.contains("[200~hello"),
        "the child reads the brackets: {said}"
    );
    asked.write(b"\r").unwrap();
    wait(&rx);
}
