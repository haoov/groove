pub(crate) mod agent;
mod palette;
mod rail;
mod session;

use groove_controllers::{AppState, Command};
use groove_gfx::{CellSize, Font, Frame, Size, TextStyle, Theme, Weight};

use crate::layout::Layout;
use crate::{Ui, theme};

/// What the renderer measured: the window and the mono cell at the theme's size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub size: Size,
    pub scale: f32,
    pub cell: CellSize,
}

/// The whole window as a display list, rebuilt every frame from state.
pub fn view(app: &AppState, ui: &Ui, metrics: Metrics) -> Frame {
    let theme = theme(app);
    let layout = Layout::new(metrics.size, metrics.scale);
    let mut frame = Frame::new(metrics.size, theme.palette.base);
    rail::draw(&mut frame, app, &theme, &layout);
    session::draw(&mut frame, app, &theme, &layout);
    agent::draw(&mut frame, app, &theme, &layout);
    if let Some(palette) = &ui.palette {
        palette::draw(&mut frame, app, palette, &theme, &layout);
    }
    frame
}

/// The commands a new window size implies: every agent's grid to the pane's grid.
pub fn layout_commands(app: &AppState, metrics: Metrics) -> Vec<Command> {
    let layout = Layout::new(metrics.size, metrics.scale);
    let (cols, rows) = layout.agent_grid(metrics.cell);
    app.agent
        .agents
        .iter()
        .filter(|(_, agent)| {
            agent
                .terminal
                .as_ref()
                .is_some_and(|t| t.size() != (cols, rows))
        })
        .map(|(session, _)| {
            Command::Agent(groove_controllers::agent::Command::Resize {
                session: session.clone(),
                cols,
                rows,
            })
        })
        .collect()
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
