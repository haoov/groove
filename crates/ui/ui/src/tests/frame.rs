use groove_controllers::AppState;
use groove_gfx::{CellSize, Palette, Size};
use groove_types::{Rgb, Screen, ScreenCell};

use crate::input::{Key, Modifiers, encode};
use crate::layout::{Layout, Split};
use crate::tests::{metrics, press};
use crate::tokens::Tokens;
use crate::widget::grid_of;
use crate::{Ui, view};

fn texts(frame: &groove_gfx::Frame) -> Vec<String> {
    frame
        .layers()
        .iter()
        .flat_map(|l| l.texts.iter().map(|t| t.text.clone()))
        .collect()
}

#[test]
fn with_nothing_open_the_board_is_the_window_and_says_how_to_start() {
    let (frame, _) = view(
        &AppState::default(),
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert_eq!(frame.layers().len(), 1);
    let rail = &frame.layers()[0].quads[0];
    assert_eq!(rail.rect.w, 220.0);
    assert_eq!(
        rail.color,
        Palette::LATTE.crust,
        "the chrome sits under the work"
    );
    let texts = texts(&frame);
    assert!(texts.iter().any(|t| t == "Board"));
    assert!(texts.iter().any(|t| t == "LIVE"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t.starts_with("nothing open")),
        "{texts:?}"
    );
}

#[test]
fn hidpi_scales_the_layout() {
    let (frame, _) = view(
        &AppState::default(),
        &Ui::default(),
        metrics(2560, 1600, 2.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert_eq!(frame.layers()[0].quads[0].rect.w, 440.0);
    assert_eq!(
        frame.layers()[0].texts[0].style.size,
        26.0,
        "13 logical, doubled"
    );
}

#[test]
fn the_agent_pane_grid_follows_the_cell_size() {
    let tokens = Tokens::new(1.0);
    let layout = Layout::new(Size::new(1280, 800), &tokens, Split::default(), false);
    let (cols, rows) = layout.agent_grid(
        &tokens,
        CellSize {
            width: 8.0,
            height: 17.0,
        },
    );
    let pane = layout.agent;
    assert_eq!(pane.w, Split::default().agent, "a width, not a share");
    assert_eq!(cols, ((pane.w - 16.0) / 8.0).floor() as u16);
    assert_eq!(rows, ((pane.h - 16.0) / 17.0).floor() as u16);
}

#[test]
fn a_screen_becomes_a_grid_with_a_swapped_cursor_cell() {
    let fg = Rgb::hex(0xcdd6f4);
    let cell = |ch: char| ScreenCell {
        ch,
        fg,
        bg: None,
        bold: false,
        spacer: false,
    };
    let mut cells = vec![cell(' '); 6];
    cells[0] = cell('a');
    cells[1] = ScreenCell {
        ch: '日',
        ..cell('日')
    };
    cells[2] = ScreenCell {
        spacer: true,
        ..cell(' ')
    };
    let screen = Screen {
        cols: 3,
        rows: 2,
        cells,
        cursor: Some((0, 1)),
    };
    let grid = grid_of(&screen, 10.0, 20.0, 12.5, Palette::MOCHA.base);
    assert_eq!((grid.cols, grid.rows), (3, 2));
    assert_eq!(grid.cell(0, 0).ch, 'a');
    assert_eq!(grid.cell(2, 0).ch, groove_gfx::WIDE_SPACER);
    let cursor = grid.cell(0, 1);
    assert_eq!(cursor.bg, groove_gfx::Color::hex(0xcdd6f4));
    assert_eq!(cursor.fg, Palette::MOCHA.base);
}

#[test]
fn keys_encode_as_a_terminal_sends_them() {
    let plain = Modifiers::default();
    let ctrl = Modifiers {
        ctrl: true,
        ..plain
    };
    assert_eq!(encode(Key::Char('a'), plain).unwrap(), b"a");
    assert_eq!(encode(Key::Char('é'), plain).unwrap(), "é".as_bytes());
    assert_eq!(encode(Key::Char('c'), ctrl).unwrap(), vec![0x03]);
    assert_eq!(encode(Key::Enter, plain).unwrap(), b"\r");
    assert_eq!(encode(Key::Escape, plain).unwrap(), b"\x1b");
    assert_eq!(encode(Key::Up, plain).unwrap(), b"\x1b[A");
    assert_eq!(encode(Key::Backspace, plain).unwrap(), b"\x7f");
    assert_eq!(
        encode(
            Key::Tab,
            Modifiers {
                shift: true,
                ..plain
            }
        )
        .unwrap(),
        b"\x1b[Z"
    );
    assert!(encode(Key::Char('1'), ctrl).is_none());
}

#[test]
fn chords_are_grooves_and_the_rest_is_the_agents() {
    let app = AppState::default();
    let mut ui = Ui::default();
    let chord = Modifiers {
        ctrl: true,
        shift: true,
        alt: false,
    };
    let opened = press(Key::Char('P'), chord, &mut ui, &app);
    assert_eq!(opened.len(), 1, "opening refreshes the pool");
    assert_eq!(opened[0].id(), "session.list_repos");
    assert!(ui.palette.is_some());
    let (frame, _) = view(
        &app,
        &ui,
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert_eq!(frame.layers().len(), 2);
    assert!(
        press(Key::Char('a'), Modifiers::default(), &mut ui, &app).is_empty(),
        "a typed key never leaks past the palette"
    );
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(ui.palette.is_none());

    let open = press(Key::Char('n'), chord, &mut ui, &app);
    assert_eq!(open[0].id(), "session.open_explorer");
    assert!(
        press(Key::Char('w'), chord, &mut ui, &app).is_empty(),
        "nothing selected, nothing to close"
    );
    assert!(
        press(Key::Char('a'), Modifiers::default(), &mut ui, &app).is_empty(),
        "no session, no agent to type into"
    );
}
