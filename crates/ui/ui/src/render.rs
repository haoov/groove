//! The one owner of a `Frame`, and what a new window size asks of the agents.

use groove_controllers::{AppState, Command, agent};
use groove_gfx::{Fonts, Frame};

use crate::ctx::{Ctx, Drawn};
use crate::hit::Hits;
use crate::layout::Layout;
use crate::views::session::{Tab, resources};
use crate::views::{board, overlays, rail, session, settings, splitter};
use crate::{Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::style::Styles;

/// The whole window as a display list, rebuilt every frame from state.
pub fn view(app: &AppState, ui: &Ui, metrics: Metrics, fonts: &mut Fonts) -> (Frame, Hits) {
    let tokens = metrics.tokens();
    let styles = Styles::new(app.config.theme(), tokens);
    let mut frame = Frame::new(metrics.size, styles.ground());
    let mut hits = Hits::default();
    {
        let drawn = Drawn {
            layout: Layout::of(metrics, ui),
            hits: &mut hits,
        };
        let theme = app.config.theme();
        let mut ctx = Ctx::new(theme, metrics, drawn, &mut frame, fonts, ui.hover.clone());
        if ui.settings.open {
            settings::draw(&mut ctx, app, ui);
        } else {
            rail::draw(&mut ctx, app, ui);
            match ui.showing(app) {
                Surface::Session => session::draw(&mut ctx, app, ui),
                Surface::Board => board::draw(&mut ctx, app, ui),
            }
            splitter::draw(&mut ctx, ui.showing(app));
        }
        if let Some(menu) = ui.menu() {
            overlays::actions::draw(&mut ctx, menu);
        }
        if let Some(palette) = ui.palette() {
            overlays::palette::draw(&mut ctx, app, palette);
        }
        if let Some(crate::Overlay::Scope(scoping)) = &ui.overlay {
            overlays::scope::draw(&mut ctx, app, ui, scoping);
        }
    }
    (frame, hits)
}

/// What the frame needs before it draws: the reads it lacks, every terminal fitted to its pane.
pub fn frame_commands(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<Command> {
    let mut out = walks(app, ui);
    out.extend(notes(app));
    out.extend(log(app, ui));
    out.extend(fitted(app, ui, metrics));
    out.extend(shells_fitted(app, ui, metrics));
    out.extend(login_fitted(app, ui, metrics));
    out.extend(schemas(app, ui));
    let up = ui.showing(app) == Surface::Session && ui.session.tab == Tab::Resources;
    out.extend(resources::wants(app, up, &ui.session.resources));
    out.extend(resources::tab_wants(
        app,
        up,
        &ui.session.resources,
        metrics.now,
    ));
    let listed = up && ui.session.resources.tab().is_none();
    if app
        .cluster
        .store
        .due()
        .is_some_and(|due| due <= metrics.now)
        && listed
    {
        let age = groove_controllers::cluster::Command::Age { now: metrics.now };
        out.push(Command::Cluster(age));
    }
    out
}

/// When the Resources panel next changes on its own: a time cell turns, a pod's usage is due.
pub fn ages_due(app: &AppState, ui: &Ui) -> Option<groove_types::Timestamp> {
    let up = ui.showing(app) == Surface::Session && ui.session.tab == Tab::Resources;
    let held = &ui.session.resources;
    let listed = held.tab().is_none().then(|| app.cluster.store.due());
    let due = [listed.flatten(), resources::tab_due(app, held)];
    up.then(|| due.into_iter().flatten().min()).flatten()
}

/// Each source's properties, read once while Providers shows its mapping.
fn schemas(app: &AppState, ui: &Ui) -> Vec<Command> {
    let shown =
        ui.settings.section == settings::Section::Providers || !ui.settings.search.is_empty();
    if !ui.settings.open || !shown {
        return Vec::new();
    }
    let on = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let unread = |one: &groove_types::ProviderId| {
        app.config.schema(*one).is_none() && !app.config.reading.contains(one)
    };
    let read = |one| Command::Config(groove_controllers::config::Command::ReadSchema(one));
    on.into_iter().filter(unread).map(read).collect()
}

/// The sign-in's grid to Setup's pane, while Settings shows it.
fn login_fitted(app: &AppState, ui: &Ui, metrics: Metrics) -> Option<Command> {
    let terminal = app.agent.login.as_ref().filter(|_| ui.settings.open)?;
    let tokens = metrics.tokens();
    let pane = settings::login_pane(metrics.size.rect(), &tokens);
    let (cols, rows) = crate::layout::grid_in(pane, &tokens, metrics.cell);
    let resize = groove_controllers::config::Command::ResizeLogin { cols, rows };
    (terminal.size() != (cols, rows)).then_some(Command::Config(resize))
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
fn notes(app: &AppState) -> Vec<Command> {
    match crate::views::session::files::needs_notes(app) {
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

/// The open section's terminals, each to the grid its pane now holds.
fn shells_fitted(app: &AppState, ui: &Ui, metrics: Metrics) -> Vec<Command> {
    let Some(session) = app.session.selected.clone().filter(|_| ui.session.manual) else {
        return Vec::new();
    };
    let Some(shells) = app.shell.shells(&session) else {
        return Vec::new();
    };
    let (tokens, layout) = (metrics.tokens(), Layout::of(metrics, ui));
    let shown = shells.shown();
    let panes = layout.shell_panes(&tokens, shown.len());
    shown
        .iter()
        .zip(panes)
        .filter_map(|(shell, pane)| {
            let (cols, rows) = crate::layout::grid_in(pane, &tokens, metrics.cell);
            let terminal = shell.terminal.as_ref()?;
            (terminal.size() != (cols, rows)).then(|| {
                let (session, id) = (session.clone(), shell.id);
                Command::Shell(groove_controllers::shell::Command::Resize {
                    session,
                    id,
                    cols,
                    rows,
                })
            })
        })
        .collect()
}
