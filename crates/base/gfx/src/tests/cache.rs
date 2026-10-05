//! What the text pass keeps between frames, so a still window shapes nothing twice.

use crate::{
    Cell, CellGrid, Color, Family, Font, Fonts, Frame, Palette, Renderer, Size, TextStyle, Weight,
};

const WINDOW: Size = Size {
    width: 400,
    height: 200,
};

fn renderer() -> Renderer {
    Renderer::headless(WINDOW, Fonts::embedded()).expect("a GPU adapter")
}

fn style() -> TextStyle {
    TextStyle {
        font: Font::Mono,
        weight: Weight::Regular,
        size: 13.0,
        color: Palette::MOCHA.text,
    }
}

/// Lines of chrome text and a small terminal grid, the same every frame.
fn frame() -> Frame {
    let mut frame = Frame::new(WINDOW, Palette::MOCHA.base);
    for row in 0..8 {
        let text = format!("fn name_{row}(value: usize) -> usize");
        frame.text(text, 4.0, row as f32 * 18.0, 18.0, style());
    }
    let mut grid = CellGrid::new(4.0, 150.0, 20, 2, 13.0);
    for (at, ch) in "the quick brown fox".chars().enumerate() {
        grid.set(at, 0, cell(ch));
    }
    frame.grid(grid);
    frame
}

fn cell(ch: char) -> Cell {
    Cell {
        ch,
        fg: Color::WHITE,
        bg: Color::TRANSPARENT,
        bold: false,
    }
}

#[test]
fn a_frame_drawn_again_shapes_nothing_new() {
    let mut renderer = renderer();
    renderer.render(&frame()).expect("the first frame");
    let first = renderer.cached();
    assert!(first.lines > 0 && first.glyphs > 0, "{first:?}");
    assert_eq!(
        first.shaped,
        (first.lines + first.glyphs) as u64,
        "the first frame shapes what it holds"
    );
    for _ in 0..3 {
        renderer.render(&frame()).expect("the same frame again");
    }
    assert_eq!(renderer.cached(), first, "three more frames shape nothing");
}

#[test]
fn text_the_frame_has_not_drawn_before_is_shaped_once() {
    let mut renderer = renderer();
    renderer.render(&frame()).expect("the first frame");
    let first = renderer.cached();
    let mut more = frame();
    more.text("a line nothing drew yet", 4.0, 180.0, 18.0, style());
    renderer.render(&more).expect("one line more");
    let after = renderer.cached();
    assert_eq!(after.lines, first.lines + 1, "one line more");
    assert_eq!(after.shaped, first.shaped + 1, "shaped once, and only it");
    assert_eq!(after.glyphs, first.glyphs, "the grid is unchanged");
}

#[test]
fn a_mono_family_changed_shapes_the_frame_again_in_it() {
    let mut renderer = renderer();
    renderer.render(&frame()).expect("the first frame");
    let first = renderer.cached();
    renderer.set_mono(Family::Lilex);
    renderer
        .render(&frame())
        .expect("the frame in the new family");
    let after = renderer.cached();
    assert_eq!(after.shaped, first.shaped * 2, "everything shaped again");
}
