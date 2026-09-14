//! Renders fixtures offscreen and compares them to `tests/golden/*.png`.
//! `GROOVE_GOLDEN=update` rewrites them; a failure writes `target/golden/<name>.png`.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use super::readback::Image;
use crate::{
    CellGrid, Color, Font, Fonts, Frame, Icon, Palette, Rect, Renderer, Size, TextStyle, Weight,
};

const MAX_CHANNEL_DIFF: u8 = 3;
const MAX_DIFFERENT_PIXELS: f64 = 0.002;

fn renderer(size: Size) -> Renderer {
    Renderer::headless(size, Fonts::embedded()).expect("a GPU adapter")
}

fn style(size: f32, weight: Weight, color: Color) -> TextStyle {
    TextStyle {
        font: Font::Sans,
        weight,
        size,
        color,
    }
}

#[test]
fn a_quad_lands_on_its_hex_value() {
    let mut r = renderer(Size::new(16, 16));
    let mut frame = Frame::new(Size::new(16, 16), Palette::MOCHA.base);
    frame.quad(Rect::new(4.0, 4.0, 8.0, 8.0), Palette::MOCHA.peach);
    let image = r.snapshot(&frame).expect("snapshot");
    assert_close(image.pixel(0, 0), [0x1e, 0x1e, 0x2e, 255]);
    assert_close(image.pixel(8, 8), [0xfa, 0xb3, 0x87, 255]);
}

#[test]
fn a_clip_cuts_a_quad() {
    let mut r = renderer(Size::new(16, 16));
    let mut frame = Frame::new(Size::new(16, 16), Color::BLACK);
    frame.clipped(Rect::new(0.0, 0.0, 8.0, 16.0), |f| {
        f.quad(Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE);
    });
    let image = r.snapshot(&frame).expect("snapshot");
    assert_close(image.pixel(7, 8), [255, 255, 255, 255]);
    assert_close(image.pixel(8, 8), [0, 0, 0, 255]);
}

#[test]
fn a_later_layer_covers_text() {
    let mut r = renderer(Size::new(64, 24));
    let mut frame = Frame::new(Size::new(64, 24), Color::BLACK);
    frame.text(
        "MMMMMM",
        2.0,
        0.0,
        24.0,
        style(14.0, Weight::Bold, Color::WHITE),
    );
    frame.layer();
    frame.quad(Rect::new(0.0, 0.0, 64.0, 24.0), Color::hex(0xff0000));
    let image = r.snapshot(&frame).expect("snapshot");
    let lit = (0..64).flat_map(|x| (0..24).map(move |y| (x, y)));
    assert!(
        lit.map(|(x, y)| image.pixel(x, y))
            .all(|p| p[1] < 8 && p[2] < 8)
    );
}

#[test]
fn text_renders_glyphs() {
    let mut r = renderer(Size::new(64, 24));
    let mut frame = Frame::new(Size::new(64, 24), Color::BLACK);
    frame.text(
        "Groove",
        2.0,
        0.0,
        24.0,
        style(14.0, Weight::Regular, Color::WHITE),
    );
    let image = r.snapshot(&frame).expect("snapshot");
    let lit = (0..64)
        .flat_map(|x| (0..24).map(move |y| (x, y)))
        .filter(|&(x, y)| image.pixel(x, y)[0] > 128)
        .count();
    assert!(lit > 40, "{lit} lit pixels");
}

#[test]
fn icons_match_golden() {
    let size = Size::new(240, 84);
    let p = Palette::MOCHA;
    let mut frame = Frame::new(size, p.base);
    let icons = [
        Icon::Flag,
        Icon::Compass,
        Icon::Eye,
        Icon::Kanban,
        Icon::Gear,
        Icon::Notch,
    ];
    for (i, icon) in icons.iter().enumerate() {
        let x = 8.0 + i as f32 * 36.0;
        frame.icon(Rect::new(x, 8.0, 24.0, 24.0), *icon, p.text);
        frame.icon(Rect::new(x, 40.0, 16.0, 16.0), *icon, p.peach);
    }
    for turn in 0..4u8 {
        let x = 8.0 + f32::from(turn) * 24.0;
        frame.icon_turned(
            Rect::new(x, 62.0, 16.0, 16.0),
            Icon::Notch,
            turn * 2,
            p.blue,
        );
    }
    compare("icons", size, &frame);
}

#[test]
fn chrome_matches_golden() {
    let size = Size::new(320, 120);
    let p = Palette::MOCHA;
    let mut frame = Frame::new(size, p.base);
    frame.quad(Rect::new(0.0, 0.0, 320.0, 28.0), p.mantle);
    frame.quad(Rect::new(0.0, 28.0, 320.0, 1.0), p.surface0);
    frame.text(
        "TASKS2-4244 · paxone cnpg",
        8.0,
        0.0,
        28.0,
        style(12.0, Weight::Medium, p.text),
    );
    frame.text(
        "waiting",
        260.0,
        0.0,
        28.0,
        style(11.0, Weight::Regular, p.peach),
    );
    frame.quad(Rect::new(8.0, 40.0, 120.0, 24.0), p.surface0);
    frame.border(Rect::new(8.0, 40.0, 120.0, 24.0), p.surface1);
    frame.text(
        "Commit",
        16.0,
        40.0,
        24.0,
        style(12.0, Weight::SemiBold, p.text),
    );
    frame.clipped(Rect::new(8.0, 72.0, 100.0, 40.0), |f| {
        f.text(
            "clipped text that runs past its box",
            8.0,
            72.0,
            20.0,
            style(12.0, Weight::Regular, p.subtext0),
        );
        f.quad(Rect::new(0.0, 96.0, 400.0, 8.0), p.green);
    });
    frame.text(
        "main",
        140.0,
        40.0,
        24.0,
        TextStyle {
            font: Font::Mono,
            ..style(12.5, Weight::Regular, p.lavender)
        },
    );
    compare("chrome", size, &frame);
}

#[test]
fn grid_matches_golden() {
    let size = Size::new(240, 100);
    let p = Palette::MOCHA;
    let mut frame = Frame::new(size, p.base);
    let mut grid = CellGrid::new(8.0, 8.0, 28, 5, 12.5);
    grid.write(0, 0, "┏━━━━━━━━━━━━━━━━━━━━━━━━━━┓", p.blue, false);
    grid.write(0, 1, "┃", p.blue, false);
    grid.write(2, 1, "$ cargo test -p groove", p.text, false);
    grid.write(27, 1, "┃", p.blue, false);
    grid.write(0, 2, "┃", p.blue, false);
    grid.write(2, 2, "ok", p.green, true);
    grid.write(5, 2, "▁▂▃▄▅▆▇█ ░▒▓ ▌▐", p.peach, false);
    grid.write(27, 2, "┃", p.blue, false);
    grid.write(0, 3, "┣━━━━━━━━━━┳━━━━━━━━━━━━━━━┫", p.blue, false);
    grid.write(0, 4, "┗━━━━━━━━━━┻━━━━━━━━━━━━━━━┛", p.blue, false);
    for col in 2..4 {
        let mut c = *grid.cell(col, 2);
        c.bg = p.surface1;
        grid.set(col, 2, c);
    }
    frame.grid(grid);
    compare("grid", size, &frame);
}

fn compare(name: &str, size: Size, frame: &Frame) {
    let image = renderer(size).snapshot(frame).expect("snapshot");
    let golden = golden_path(name);
    if std::env::var("GROOVE_GOLDEN").is_ok_and(|v| v == "update") {
        write_png(&golden, &image);
        return;
    }
    let Some(expected) = read_png(&golden) else {
        write_png(&actual_path(name), &image);
        panic!(
            "no golden {}; run with GROOVE_GOLDEN=update",
            golden.display()
        );
    };
    assert_eq!(expected.size, image.size, "golden size");
    let different = expected
        .rgba
        .iter()
        .zip(&image.rgba)
        .map(|(a, b)| a.abs_diff(*b))
        .collect::<Vec<u8>>()
        .chunks(4)
        .filter(|px| px.iter().any(|d| *d > MAX_CHANNEL_DIFF))
        .count();
    let share = different as f64 / f64::from(size.width * size.height);
    if share > MAX_DIFFERENT_PIXELS {
        let actual = actual_path(name);
        write_png(&actual, &image);
        panic!(
            "{name}: {different} pixels differ; see {}",
            actual.display()
        );
    }
}

fn assert_close(got: [u8; 4], want: [u8; 4]) {
    let close = got
        .iter()
        .zip(want)
        .all(|(g, w)| g.abs_diff(w) <= MAX_CHANNEL_DIFF);
    assert!(close, "got {got:?}, want {want:?}");
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/golden/{name}.png"))
}

fn actual_path(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/golden");
    std::fs::create_dir_all(&dir).expect("target dir");
    dir.join(format!("{name}.png"))
}

fn write_png(path: &PathBuf, image: &Image) {
    let file = File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(file, image.size.width, image.size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&image.rgba).expect("png data");
}

fn read_png(path: &PathBuf) -> Option<Image> {
    let file = File::open(path).ok()?;
    let mut reader = png::Decoder::new(BufReader::new(file))
        .read_info()
        .expect("png info");
    let mut rgba = vec![0; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut rgba).expect("png frame");
    rgba.truncate(info.buffer_size());
    Some(Image {
        size: Size::new(info.width, info.height),
        rgba,
    })
}
