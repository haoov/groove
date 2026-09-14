//! The session layout as a frame: rail, header, terminal grid, sidebar, palette.

use groove_gfx::{CellGrid, Color, Font, Frame, Palette, Rect, Renderer, Size, TextStyle, Weight};

const P: Palette = Palette::MOCHA;

/// The demo's own type scale. The app's lives in the ui crate.
struct Theme {
    text: f32,
    small: f32,
    title: f32,
    mono: f32,
}

impl Theme {
    fn dark() -> Self {
        Self {
            text: 12.0,
            small: 11.0,
            title: 14.0,
            mono: 12.5,
        }
    }
}
const RAIL: f32 = 200.0;
const HEADER: f32 = 32.0;
const SIDEBAR: f32 = 280.0;
const ROW: f32 = 26.0;

pub fn frame(renderer: &mut Renderer, size: Size, scale: f32, palette: bool) -> Frame {
    let theme = Theme::dark();
    let mut frame = Frame::new(size, P.base);
    let (w, h) = (size.width as f32, size.height as f32);
    let rail = RAIL * scale;
    let header = HEADER * scale;
    let sidebar = SIDEBAR * scale;

    draw_rail(&mut frame, &theme, scale, Rect::new(0.0, 0.0, rail, h));
    draw_header(
        &mut frame,
        &theme,
        scale,
        Rect::new(rail, 0.0, w - rail, header),
    );
    let body = Rect::new(rail, header, w - rail - sidebar, h - header);
    draw_terminal(&mut frame, renderer, &theme, scale, body);
    draw_sidebar(
        &mut frame,
        &theme,
        scale,
        Rect::new(w - sidebar, header, sidebar, h - header),
    );
    if palette {
        draw_palette(&mut frame, &theme, scale, size);
    }
    frame
}

fn sans(size: f32, weight: Weight, color: Color) -> TextStyle {
    TextStyle {
        font: Font::Sans,
        weight,
        size,
        color,
    }
}

fn draw_rail(frame: &mut Frame, theme: &Theme, scale: f32, rect: Rect) {
    frame.quad(rect, P.mantle);
    frame.quad(
        Rect::new(rect.right() - 1.0, rect.y, 1.0, rect.h),
        P.surface0,
    );
    let row = ROW * scale;
    let sessions = [
        ("TASKS2-4244 · paxone cnpg", "working", P.blue),
        ("TASKS2-4138 · rbac v2", "asks", P.peach),
        ("review !812 · overwhelm", "idle", P.overlay0),
        ("explorer · gfx probe", "exited", P.overlay0),
    ];
    for (i, (title, status, color)) in sessions.iter().enumerate() {
        let y = rect.y + 12.0 * scale + i as f32 * row;
        if i == 0 {
            frame.quad(Rect::new(rect.x, y, rect.w - 1.0, row), P.surface0);
        }
        frame.quad(
            Rect::new(
                rect.x + 8.0 * scale,
                y + row / 2.0 - 3.0 * scale,
                6.0 * scale,
                6.0 * scale,
            ),
            *color,
        );
        frame.clipped(Rect::new(rect.x, y, rect.w - 60.0 * scale, row), |f| {
            f.text(
                *title,
                rect.x + 22.0 * scale,
                y,
                row,
                sans(theme.text * scale, Weight::Medium, P.text),
            );
        });
        frame.text(
            *status,
            rect.right() - 56.0 * scale,
            y,
            row,
            sans(theme.small * scale, Weight::Regular, *color),
        );
    }
    let footer = Rect::new(rect.x, rect.bottom() - row, rect.w - 1.0, row);
    frame.quad(Rect::new(footer.x, footer.y, footer.w, 1.0), P.surface0);
    frame.text(
        "board  ·  settings",
        footer.x + 8.0 * scale,
        footer.y,
        row,
        sans(theme.small * scale, Weight::Regular, P.subtext0),
    );
}

fn draw_header(frame: &mut Frame, theme: &Theme, scale: f32, rect: Rect) {
    frame.quad(rect, P.mantle);
    frame.quad(
        Rect::new(rect.x, rect.bottom() - 1.0, rect.w, 1.0),
        P.surface0,
    );
    frame.text(
        "TASKS2-4244 · paxone-staging-gite CNPG migration",
        rect.x + 12.0 * scale,
        rect.y,
        rect.h,
        sans(theme.title * scale, Weight::SemiBold, P.text),
    );
    let tabs = [("overview", false), ("diff", true), ("editor", false)];
    let mut x = rect.right() - 300.0 * scale;
    for (tab, selected) in tabs {
        let tab_rect = Rect::new(x, rect.y + 4.0 * scale, 80.0 * scale, rect.h - 8.0 * scale);
        if selected {
            frame.quad(tab_rect, P.surface0);
        }
        frame.text(
            tab,
            x + 12.0 * scale,
            tab_rect.y,
            tab_rect.h,
            sans(
                theme.text * scale,
                Weight::Medium,
                if selected { P.text } else { P.subtext0 },
            ),
        );
        x += 90.0 * scale;
    }
}

fn draw_terminal(
    frame: &mut Frame,
    renderer: &mut Renderer,
    theme: &Theme,
    scale: f32,
    rect: Rect,
) {
    let font_size = theme.mono * scale;
    let cell = renderer.fonts().cell_size(font_size);
    let pad = 8.0 * scale;
    let cols = ((rect.w - 2.0 * pad) / cell.width).max(1.0) as usize;
    let rows = ((rect.h - 2.0 * pad) / cell.height).max(1.0) as usize;
    let mut grid = CellGrid::new(rect.x + pad, rect.y + pad, cols, rows, font_size);
    let lines = [
        ("$ cargo test -p groove-gfx", P.text, false),
        ("   Compiling groove-gfx v0.1.0", P.green, true),
        ("running 13 tests", P.text, false),
        (
            "test tests::golden::grid_matches_golden ... ok",
            P.text,
            false,
        ),
        (
            "test tests::golden::chrome_matches_golden ... ok",
            P.text,
            false,
        ),
        ("", P.text, false),
        ("┌──────────────┬──────────────────────────┐", P.blue, false),
        ("│ light frame  │ ┏━━━━━━━━━━━━━━━━━━━━━━┓ │", P.blue, false),
        ("│ ├── tees     │ ┃ heavy inside light   ┃ │", P.blue, false),
        ("│ └── stubs ╴╵ │ ┗━━━━━━━━━━━━━━━━━━━━━━┛ │", P.blue, false),
        ("└──────────────┴──────────────────────────┘", P.blue, false),
        ("▁▂▃▄▅▆▇█ ░▒▓ ▌▐ ▀▄", P.peach, false),
        ("", P.text, false),
        (
            "● Claude is working — 12 files read, 3 edited",
            P.blue,
            false,
        ),
        (
            "> ask: commit `feat(gfx): add the gfx base crate`?  [y/n]",
            P.peach,
            true,
        ),
    ];
    for (row, (line, color, bold)) in lines.iter().enumerate().take(rows) {
        grid.write(0, row, line, *color, *bold);
    }
    let last = lines.len().min(rows).saturating_sub(1);
    for col in 0..cols {
        let mut c = *grid.cell(col, last);
        c.bg = P.surface0;
        grid.set(col, last, c);
    }
    frame.clipped(rect, |f| f.grid(grid));
}

fn draw_sidebar(frame: &mut Frame, theme: &Theme, scale: f32, rect: Rect) {
    frame.quad(Rect::new(rect.x, rect.y, 1.0, rect.h), P.surface0);
    let row = ROW * scale;
    let files = [
        ("M", "crates/base/gfx/src/renderer.rs", P.yellow),
        ("A", "crates/base/gfx/src/text.rs", P.green),
        ("A", "crates/base/gfx/src/quads.rs", P.green),
        ("D", "probe/src/grid.rs", P.red),
        ("M", "Cargo.toml", P.yellow),
    ];
    frame.text(
        "files",
        rect.x + 12.0 * scale,
        rect.y,
        row,
        sans(theme.small * scale, Weight::SemiBold, P.subtext0),
    );
    for (i, (mark, path, color)) in files.iter().enumerate() {
        let y = rect.y + row * (i as f32 + 1.0);
        frame.text(
            *mark,
            rect.x + 12.0 * scale,
            y,
            row,
            TextStyle {
                font: Font::Mono,
                ..sans(theme.mono * scale, Weight::Bold, *color)
            },
        );
        frame.clipped(Rect::new(rect.x, y, rect.w, row), |f| {
            f.text(
                *path,
                rect.x + 32.0 * scale,
                y,
                row,
                TextStyle {
                    font: Font::Mono,
                    ..sans(theme.mono * scale, Weight::Regular, P.text)
                },
            );
        });
    }
    let commit = Rect::new(
        rect.x + 12.0 * scale,
        rect.y + row * 7.0,
        rect.w - 24.0 * scale,
        96.0 * scale,
    );
    frame.border(commit, P.surface1);
    frame.text(
        "feat(gfx): add the gfx base crate",
        commit.x + 8.0 * scale,
        commit.y,
        row,
        sans(theme.text * scale, Weight::Regular, P.text),
    );
    let button = Rect::new(
        commit.right() - 80.0 * scale,
        commit.bottom() + 8.0 * scale,
        80.0 * scale,
        row,
    );
    frame.quad(button, P.blue);
    frame.text(
        "Commit",
        button.x + 18.0 * scale,
        button.y,
        row,
        sans(theme.text * scale, Weight::SemiBold, P.crust),
    );
}

fn draw_palette(frame: &mut Frame, theme: &Theme, scale: f32, size: Size) {
    frame.layer();
    frame.quad(size.rect(), P.crust.with_alpha(160));
    let w = 560.0 * scale;
    let row = ROW * scale;
    let rect = Rect::new(
        (size.width as f32 - w) / 2.0,
        120.0 * scale,
        w,
        row * 6.0 + 16.0 * scale,
    );
    frame.quad(rect, P.mantle);
    frame.border(rect, P.surface1);
    frame.text(
        "> commit",
        rect.x + 12.0 * scale,
        rect.y + 8.0 * scale,
        row,
        TextStyle {
            font: Font::Mono,
            ..sans(theme.mono * scale, Weight::Regular, P.text)
        },
    );
    frame.quad(
        Rect::new(rect.x, rect.y + row + 8.0 * scale, rect.w, 1.0),
        P.surface0,
    );
    let items = [
        "commit the selected worktree",
        "commit and push",
        "open the commit box",
        "copy the commit title",
    ];
    for (i, item) in items.iter().enumerate() {
        let y = rect.y + 8.0 * scale + row * (i as f32 + 1.0);
        if i == 0 {
            frame.quad(Rect::new(rect.x + 1.0, y, rect.w - 2.0, row), P.surface0);
        }
        frame.text(
            *item,
            rect.x + 12.0 * scale,
            y,
            row,
            sans(theme.text * scale, Weight::Regular, P.text),
        );
    }
}
