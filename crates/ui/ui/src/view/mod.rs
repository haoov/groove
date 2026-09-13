mod palette;
mod rail;
mod session;

use groove_controllers::AppState;
use groove_gfx::{Font, Frame, Size, TextStyle, Theme, Weight};

use crate::layout::Layout;
use crate::{Ui, theme};

/// The whole window as a display list, rebuilt every frame from state.
pub fn view(app: &AppState, ui: &Ui, size: Size, scale: f32) -> Frame {
    let theme = theme(app);
    let layout = Layout::new(size, scale);
    let mut frame = Frame::new(size, theme.palette.base);
    rail::draw(&mut frame, app, &theme, &layout);
    session::draw(&mut frame, app, &theme, &layout);
    if ui.palette_open {
        palette::draw(&mut frame, &theme, &layout);
    }
    frame
}

pub(crate) fn sans(
    layout: &Layout,
    size: f32,
    weight: Weight,
    color: groove_gfx::Color,
) -> TextStyle {
    TextStyle {
        font: Font::Sans,
        weight,
        size: layout.px(size),
        color,
    }
}

pub(crate) fn mono(theme: &Theme, layout: &Layout, color: groove_gfx::Color) -> TextStyle {
    TextStyle {
        font: Font::Mono,
        weight: Weight::Regular,
        size: layout.px(theme.mono),
        color,
    }
}
