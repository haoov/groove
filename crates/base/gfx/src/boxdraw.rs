//! Box-drawing and block glyphs as rectangles that tile the cell exactly.
//! Dashed variants draw solid; double and rounded glyphs stay on the font.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    None,
    Light,
    Heavy,
}

use Arm::{Heavy as H, Light as L, None as N};

/// A rectangle in absolute pixels, with the alpha to draw it at.
pub(crate) type BoxQuad = ([f32; 4], f32);

/// The quads for one cell, or `None` when the glyph belongs to the font.
pub(crate) fn quads(ch: char, x0: f32, y0: f32, w: f32, h: f32, size: f32) -> Option<Vec<BoxQuad>> {
    if let Some(q) = block(ch, x0, y0, w, h) {
        return Some(q);
    }
    let arms = arms(ch)?;
    let light = (size / 12.0).round().max(1.0);
    let heavy = light * 2.0;
    let cx = (x0 + w / 2.0).round();
    let cy = (y0 + h / 2.0).round();
    let (x1, y1) = (x0 + w, y0 + h);

    let mut out = Vec::new();
    // Up, right, down, left.
    for (side, arm) in arms.iter().enumerate() {
        let t = match arm {
            N => continue,
            L => light,
            H => heavy,
        };
        let half = t / 2.0;
        let r = match side {
            0 => [cx - half, y0, cx + half, cy + half],
            1 => [cx - half, cy - half, x1, cy + half],
            2 => [cx - half, cy - half, cx + half, y1],
            _ => [x0, cy - half, cx + half, cy + half],
        };
        out.push((r, 1.0));
    }
    Some(out)
}

/// Full, half, eighth and shaded blocks.
fn block(ch: char, x0: f32, y0: f32, w: f32, h: f32) -> Option<Vec<BoxQuad>> {
    let all = [x0, y0, x0 + w, y0 + h];
    let lower = |n: u32| [x0, y0 + h * (1.0 - n as f32 / 8.0), x0 + w, y0 + h];
    let left = |n: u32| [x0, y0, x0 + w * (n as f32 / 8.0), y0 + h];
    let c = ch as u32;
    let q = match ch {
        '░' => (all, 0.25),
        '▒' => (all, 0.5),
        '▓' => (all, 0.75),
        '▀' => ([x0, y0, x0 + w, y0 + h / 2.0], 1.0),
        '▐' => ([x0 + w / 2.0, y0, x0 + w, y0 + h], 1.0),
        // U+2581..U+2588: lower one eighth up to the full block.
        '▁'..='█' => (lower(c - 0x2580), 1.0),
        // U+2589..U+258F: left seven eighths down to one.
        '▉'..='▏' => (left(0x2590 - c), 1.0),
        _ => return None,
    };
    Some(vec![q])
}

/// Arm weights as up, right, down, left.
fn arms(ch: char) -> Option<[Arm; 4]> {
    let a = match ch {
        '─' | '┄' | '┈' | '╌' => [N, L, N, L],
        '━' | '┅' | '┉' | '╍' => [N, H, N, H],
        '│' | '┆' | '┊' | '╎' => [L, N, L, N],
        '┃' | '┇' | '┋' | '╏' => [H, N, H, N],
        '┌' => [N, L, L, N],
        '┐' => [N, N, L, L],
        '└' => [L, L, N, N],
        '┘' => [L, N, N, L],
        '┍' => [N, H, L, N],
        '┎' => [N, L, H, N],
        '┏' => [N, H, H, N],
        '┑' => [N, N, L, H],
        '┒' => [N, N, H, L],
        '┓' => [N, N, H, H],
        '┕' => [L, H, N, N],
        '┖' => [H, L, N, N],
        '┗' => [H, H, N, N],
        '┙' => [L, N, N, H],
        '┚' => [H, N, N, L],
        '┛' => [H, N, N, H],
        '├' => [L, L, L, N],
        '┣' => [H, H, H, N],
        '┤' => [L, N, L, L],
        '┫' => [H, N, H, H],
        '┬' => [N, L, L, L],
        '┳' => [N, H, H, H],
        '┴' => [L, L, N, L],
        '┻' => [H, H, N, H],
        '┼' => [L, L, L, L],
        '╋' => [H, H, H, H],
        '╴' => [N, N, N, L],
        '╵' => [L, N, N, N],
        '╶' => [N, L, N, N],
        '╷' => [N, N, L, N],
        '╸' => [N, N, N, H],
        '╹' => [H, N, N, N],
        '╺' => [N, H, N, N],
        '╻' => [N, N, H, N],
        _ => return None,
    };
    Some(a)
}
