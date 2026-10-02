use crate::{
    Align, Cell, CellGrid, Color, Edges, Font, Frame, Palette, Rect, Size, TextStyle, Weight,
    boxdraw,
};

#[test]
fn a_colour_keeps_the_channels_its_hex_names() {
    let c = Color::hex(0x1e1e2e);
    assert_eq!((c.r, c.g, c.b, c.a), (0x1e, 0x1e, 0x2e, 255));
}

#[test]
fn a_colour_reaches_the_gpu_linear_and_with_its_alpha() {
    assert_eq!(Color::BLACK.linear(), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(Color::WHITE.linear(), [1.0, 1.0, 1.0, 1.0]);
    let mid = Color::rgb(128, 128, 128).linear()[0];
    assert!((mid - 0.2158).abs() < 0.001, "{mid}");
    assert_eq!(Color::WHITE.with_alpha(0).linear()[3], 0.0);
}

#[test]
fn two_rects_meet_in_what_they_share_and_nothing_when_they_do_not() {
    let a = Rect::new(0.0, 0.0, 10.0, 10.0);
    let b = Rect::new(5.0, 5.0, 10.0, 10.0);
    assert_eq!(a.intersect(b), Rect::new(5.0, 5.0, 5.0, 5.0));
    assert!(a.intersect(Rect::new(20.0, 20.0, 1.0, 1.0)).is_empty());
}

#[test]
fn a_row_hands_out_its_ends_and_keeps_the_middle() {
    let mut row = Rect::new(0.0, 0.0, 100.0, 20.0);
    assert_eq!(row.take_left(10.0), Rect::new(0.0, 0.0, 10.0, 20.0));
    assert_eq!(row.take_right(30.0), Rect::new(70.0, 0.0, 30.0, 20.0));
    assert_eq!(row, Rect::new(10.0, 0.0, 60.0, 20.0));
    assert_eq!(row.take_left(500.0).w, 60.0);
    assert!(row.is_empty());
}

#[test]
fn a_box_sits_where_it_is_aligned_inside_the_padding() {
    let room = Rect::new(0.0, 0.0, 100.0, 40.0).pad(Edges::all(10.0));
    assert_eq!(room, Rect::new(10.0, 10.0, 80.0, 20.0));
    let centred = room.align((20.0, 10.0), Align::End, Align::Center);
    assert_eq!(centred, Rect::new(70.0, 15.0, 20.0, 10.0));
}

#[test]
fn a_rect_becomes_a_scissor_clamped_to_the_surface() {
    let b = Rect::new(5.0, 5.0, 10.0, 10.0);
    let size = Size::new(8, 8);
    assert_eq!(b.scissor(size), Some((5, 5, 3, 3)));
    assert_eq!(
        Rect::new(-2.5, -2.5, 4.0, 4.0).scissor(size),
        Some((0, 0, 2, 2))
    );
    assert_eq!(Rect::new(9.0, 9.0, 4.0, 4.0).scissor(size), None);
}

#[test]
fn frame_clips_nest_and_layers_stack() {
    let mut frame = Frame::new(Size::new(100, 100), Color::BLACK);
    let style = TextStyle {
        font: Font::Sans,
        weight: Weight::Regular,
        size: 12.0,
        color: Color::WHITE,
    };
    frame.clipped(Rect::new(10.0, 10.0, 50.0, 50.0), |f| {
        f.clipped(Rect::new(0.0, 0.0, 30.0, 30.0), |f| {
            f.quad(Rect::new(0.0, 0.0, 100.0, 100.0), Color::WHITE);
        });
        f.text("hi", 0.0, 0.0, 20.0, style);
    });
    frame.layer();
    frame.quad(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE);
    frame.quad(Rect::new(0.0, 0.0, 1.0, 1.0), Color::TRANSPARENT);

    assert_eq!(frame.layers.len(), 2);
    assert_eq!(
        frame.layers[0].quads[0].clip,
        Rect::new(10.0, 10.0, 20.0, 20.0)
    );
    assert_eq!(
        frame.layers[0].texts[0].clip,
        Rect::new(10.0, 10.0, 50.0, 50.0)
    );
    assert_eq!(frame.layers[1].quads.len(), 1);
    assert_eq!(frame.layers[1].quads[0].clip, Size::new(100, 100).rect());
}

#[test]
fn border_is_four_hairlines() {
    let mut frame = Frame::new(Size::new(10, 10), Color::BLACK);
    frame.border(Rect::new(1.0, 1.0, 8.0, 8.0), Color::WHITE);
    let quads = &frame.layers[0].quads;
    assert_eq!(quads.len(), 4);
    assert!(quads.iter().all(|q| q.rect.w == 1.0 || q.rect.h == 1.0));
}

#[test]
fn box_glyphs_tile_the_cell() {
    let (x0, y0, w, h) = (10.0, 20.0, 8.0, 16.0);
    for ch in ['┼', '━', '│', '┌', '┛', '╋', '█', '▄', '▌', '▒'] {
        let quads = boxdraw::quads(ch, x0, y0, w, h, 12.0).unwrap_or_else(|| panic!("{ch}"));
        for ([qx0, qy0, qx1, qy1], alpha) in quads {
            assert!(
                qx0 >= x0 && qy0 >= y0 && qx1 <= x0 + w && qy1 <= y0 + h,
                "{ch}"
            );
            assert!(qx1 > qx0 && qy1 > qy0, "{ch}");
            assert!(alpha > 0.0 && alpha <= 1.0);
        }
    }
    assert_eq!(
        boxdraw::quads('┼', 0.0, 0.0, 8.0, 16.0, 12.0).map(|q| q.len()),
        Some(4)
    );
    assert_eq!(boxdraw::quads('╭', 0.0, 0.0, 8.0, 16.0, 12.0), None);
    assert_eq!(boxdraw::quads('a', 0.0, 0.0, 8.0, 16.0, 12.0), None);
}

#[test]
fn grid_write_stops_at_the_edge() {
    let mut grid = CellGrid::new(0.0, 0.0, 3, 1, 12.0);
    grid.set(
        2,
        0,
        Cell {
            bg: Color::hex(0x313244),
            ..Cell::default()
        },
    );
    grid.write(1, 0, "abc", Color::WHITE, true);
    assert_eq!(grid.cell(0, 0).ch, ' ');
    assert_eq!(grid.cell(1, 0).ch, 'a');
    assert_eq!(grid.cell(2, 0).ch, 'b');
    assert!(grid.cell(2, 0).bold);
    assert_eq!(grid.cell(2, 0).bg, Color::hex(0x313244));
}

#[test]
fn palettes_carry_the_design_values() {
    assert_eq!(Palette::MOCHA.base, Color::hex(0x1e1e2e));
    assert_eq!(Palette::MOCHA.peach, Color::hex(0xfab387));
    assert_eq!(Palette::LATTE.base, Color::hex(0xeff1f5));
    assert_eq!(Palette::LATTE.text, Color::hex(0x4c4f69));
}

#[test]
fn every_icon_rasterizes_to_ink() {
    let icons = crate::icons::Icons::new();
    for icon in crate::Icon::ALL {
        let glyph = icon.glyph_id(0);
        let mask = icons
            .rasterize(glyph, 32, 32)
            .unwrap_or_else(|| panic!("{icon:?} has no shape"));
        assert_eq!(mask.len(), 32 * 32);
        assert!(
            mask.iter().any(|alpha| *alpha > 0),
            "{icon:?} rasterized empty"
        );
    }
}

#[test]
fn text_with_a_paragraph_break_and_right_to_left_words_is_measured_as_one_line() {
    let mut fonts = crate::Fonts::embedded();
    for cut in ['\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2029}'] {
        let text = format!("fix{cut}שלום");
        let width = fonts.measure(&text, Font::Mono, Weight::Regular, 13.0);
        assert!(width > 0.0, "{cut:?}");
    }
}
