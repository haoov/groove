//! The one owner of a `Frame`, and what a new window size asks of the agents.

use groove_controllers::{AppState, Command, agent};
use groove_gfx::{Fonts, Frame};

use crate::base::ctx::{Ctx, Metrics};
use crate::base::hit::Hits;
use crate::base::style::Styles;
use crate::layout::Layout;
use crate::views::{board, session, shared};
use crate::{Surface, Ui};

/// The whole window as a display list, rebuilt every frame from state.
pub fn view(app: &AppState, ui: &Ui, metrics: Metrics, fonts: &mut Fonts) -> (Frame, Hits) {
    let tokens = metrics.tokens();
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
            ui.hover.clone(),
        );
        shared::rail::draw(&mut ctx, app, ui);
        match ui.showing(app) {
            Surface::Session => session::draw(&mut ctx, app, ui),
            Surface::Board => board::draw(&mut ctx, app, ui),
        }
        shared::splitter::draw(&mut ctx, ui.showing(app));
        if let Some(menu) = ui.menu() {
            shared::actions::draw(&mut ctx, ui, menu);
        }
        if let Some(palette) = ui.palette() {
            shared::palette::draw(&mut ctx, app, palette);
        }
    }
    (frame, hits)
}

/// The commands a new window size implies: every agent's grid to the pane's grid.
pub fn layout_commands(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<Command> {
    let mut out = walks(app, ui);
    out.extend(notes(app, ui));
    out.extend(log(app, ui));
    out.extend(fitted(app, ui, metrics));
    out
}

/// The commits the sidebar's list needs before it can show them.
fn log(app: &AppState, ui: &Ui) -> Vec<Command> {
    match crate::views::session::files::needs_commits(app, ui) {
        true => vec![Command::Workspace(
            groove_controllers::workspace::Command::GetCommits,
        )],
        false => Vec::new(),
    }
}

/// The notes the sidebar's list needs before it can show them.
fn notes(app: &AppState, ui: &Ui) -> Vec<Command> {
    match crate::views::session::files::needs_notes(app, ui) {
        true => vec![Command::Delivery(
            groove_controllers::delivery::Command::GetNotes,
        )],
        false => Vec::new(),
    }
}

/// The walk the explorer needs before it can draw a tree.
fn walks(app: &AppState, ui: &Ui) -> Vec<Command> {
    match crate::views::session::files::needs_walk(app, ui) {
        true => vec![Command::Workspace(
            groove_controllers::workspace::Command::ListPaths,
        )],
        false => Vec::new(),
    }
}

/// Every agent's grid to the pane it stands in.
fn fitted(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<Command> {
    let tokens = metrics.tokens();
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
