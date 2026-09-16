//! The one place that owns a `Frame`: the window as a display list, and what a new
//! window size asks of the agents.

use groove_controllers::{AppState, Command, agent};
use groove_gfx::{Fonts, Frame};

use crate::Ui;
use crate::ctx::{Ctx, Metrics};
use crate::hit::Hits;
use crate::layout::Layout;
use crate::style::Styles;
use crate::tokens::Tokens;
use crate::views::{session, shared};

/// The whole window as a display list, rebuilt every frame from state.
pub fn view(app: &AppState, ui: &Ui, metrics: Metrics, fonts: &mut Fonts) -> (Frame, Hits) {
    let tokens = Tokens::new(metrics.scale);
    let styles = Styles::new(app.config.theme(), tokens);
    let mut frame = Frame::new(metrics.size, styles.ground());
    let mut hits = Hits::default();
    {
        let mut ctx = Ctx::new(
            app,
            metrics,
            Layout::of(metrics, ui),
            &mut frame,
            fonts,
            &mut hits,
        );
        shared::rail::draw(&mut ctx, app, ui);
        session::draw(&mut ctx, app, ui);
        shared::splitter::draw(&mut ctx);
        if let Some(palette) = &ui.palette {
            shared::palette::draw(&mut ctx, app, palette);
        }
    }
    (frame, hits)
}

/// The commands a new window size implies: every agent's grid to the pane's grid.
pub fn layout_commands(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<Command> {
    let tokens = Tokens::new(metrics.scale);
    let layout = Layout::of(metrics, ui);
    let (cols, rows) = layout.agent_grid(&tokens, metrics.cell);
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
            Command::Agent(agent::Command::Resize {
                session: session.clone(),
                cols,
                rows,
            })
        })
        .collect()
}
