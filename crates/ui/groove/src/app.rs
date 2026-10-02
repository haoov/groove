//! The window's state and what it does with it: draw, fit, point, dispatch.

mod events;

use self::events::{CLOCK_S, FIT_MS, FRAME_MS};

use std::sync::Arc;
use std::time::{Duration, Instant};

use groove_controllers::{
    AppState, Command, Env, Event, Services, TokioSpawner, agent, apply, dispatch, workspace,
};
use groove_gfx::{Renderer, Size};
use groove_types::{AttentionClass, Config, Panes, Timestamp};
use groove_ui::input::Input;
use groove_ui::{Cursor, Hits, Metrics, Split, Ui};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window};

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    state: AppState,
    ui: Ui,
    spawner: TokioSpawner,
    services: Services,
    modifiers: ModifiersState,
    /// What the last frame drew, and where the pointer is.
    hits: Hits,
    cursor: (f32, f32),
    pointer: Cursor,
    /// When the agents were last fitted to their pane.
    fitted: Instant,
    failure: Option<groove_gfx::Error>,
    explore: bool,
    started: Instant,
    /// When the paced redraw is next due.
    tick: Option<Instant>,
}

impl App {
    pub fn new(
        spawner: TokioSpawner,
        services: Services,
        env: Env,
        config: Option<Config>,
        panes: Option<Panes>,
        explore: bool,
    ) -> Self {
        let mut state = AppState::new(env);
        state.config.config = config;
        let ui = Ui {
            split: panes.map(Split::of).unwrap_or_default(),
            ..Ui::default()
        };
        Self {
            window: None,
            renderer: None,
            state,
            ui,
            spawner,
            services,
            modifiers: ModifiersState::empty(),
            hits: Hits::default(),
            cursor: (0.0, 0.0),
            pointer: Cursor::default(),
            fitted: Instant::now(),
            failure: None,
            explore,
            started: Instant::now(),
            tick: None,
        }
    }

    /// The reason the loop stopped, when it was ours.
    pub fn into_result(self) -> Result<(), Box<dyn std::error::Error>> {
        match self.failure {
            Some(e) => Err(Box::new(e)),
            None => Ok(()),
        }
    }

    fn redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Fits every agent to the pane, then draws.
    fn draw(&mut self) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        self.fit_agents(metrics);
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let (frame, hits) = groove_ui::view(&self.state, &self.ui, metrics, renderer.fonts());
        let _ = renderer.render(&frame);
        self.hits = hits;
        self.showing();
        self.wrapping();
        self.point();
        if let Some(blame) = groove_ui::input::rest(&mut self.ui, &self.state, metrics.tick) {
            dispatch(blame, &mut self.state, &self.services, &self.spawner);
        }
    }

    /// The width a note row holds, for the next frame to wrap notes to.
    fn wrapping(&mut self) {
        let cols = self.hits.wrap();
        if cols != 0 && cols != self.ui.session.note_cols {
            self.ui.session.note_cols = cols;
            self.redraw();
        }
    }

    /// The rows the frame drew, so their files can take their colours.
    fn showing(&mut self) {
        let rows = self.hits.shown();
        if rows == self.state.workspace.showing {
            return;
        }
        let command = Command::Workspace(workspace::Command::Show { rows });
        dispatch(command, &mut self.state, &self.services, &self.spawner);
    }

    /// How long the window may sleep before it redraws itself.
    fn pace(&self) -> Option<Duration> {
        let moving = !self.state.pending.is_empty()
            || self
                .state
                .agent
                .agents
                .iter()
                .any(|(_, agent)| agent.activity.class() == AttentionClass::Moving);
        if moving {
            return Some(Duration::from_millis(FRAME_MS));
        }
        let tick = self.started.elapsed().as_millis() as u64;
        if let Some(left) = groove_ui::input::resting(&self.ui, tick) {
            return Some(Duration::from_millis(left));
        }
        let waiting =
            !self.state.agent.agents.is_empty() || groove_controllers::delivery::polls(&self.state);
        waiting.then(|| Duration::from_secs(CLOCK_S))
    }

    /// Every agent's grid to its pane, throttled while a split is dragged.
    fn fit_agents(&mut self, metrics: Metrics) {
        let waiting = self.fitted.elapsed() < Duration::from_millis(FIT_MS);
        if self.ui.dragging() && waiting {
            return;
        }
        self.fitted = Instant::now();
        for command in groove_ui::layout_commands(&self.state, &self.ui, metrics) {
            dispatch(command, &mut self.state, &self.services, &self.spawner);
        }
    }

    /// The pointer follows what is under it, and changes only when it must.
    fn point(&mut self) {
        let (x, y) = self.cursor;
        let wanted = groove_ui::input::cursor(&self.ui, &self.hits, x, y);
        if wanted == self.pointer {
            return;
        }
        self.pointer = wanted;
        if let Some(window) = &self.window {
            window.set_cursor(icon_of(wanted));
        }
    }

    /// One input, then whatever it asks of the services.
    fn input(&mut self, input: Input) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        self.state.acted_at = groove_types::Timestamp::now();
        let typed = matches!(input, Input::Key { .. } | Input::Paste(_));
        let commands =
            groove_ui::input::handle(input, &mut self.ui, &self.state, &self.hits, metrics);
        for command in commands {
            dispatch(command, &mut self.state, &self.services, &self.spawner);
        }
        if typed {
            groove_ui::input::follow(&mut self.ui, &self.state, &self.hits, metrics);
        }
        self.clock();
        self.redraw();
    }

    /// The clock on the task being worked, and the poll on the open MRs.
    fn clock(&mut self) {
        let now = groove_types::Timestamp::now();
        groove_controllers::task::time::tick(&mut self.state, &self.services, &self.spawner, now);
        groove_controllers::delivery::poll(&mut self.state, &self.services, &self.spawner, now);
    }

    fn metrics(&mut self) -> Option<Metrics> {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return None;
        };
        let scale = window.scale_factor() as f32;
        let config = &self.state.config;
        let (text, code, terminal) = (
            config.text_size(),
            config.code_size(),
            config.terminal_size(),
        );
        let fonts = renderer.fonts();
        Some(Metrics {
            size: size_of(window),
            scale,
            text,
            code,
            terminal,
            cell: fonts.cell_size(terminal * scale),
            advance: fonts.cell_size(code * scale).width,
            tick: self.started.elapsed().as_millis() as u64,
            now: Timestamp::now(),
        })
    }

    /// Every agent gets SIGTERM before the window goes; the sessions stay on the rail.
    fn end_agents(&mut self) {
        let sessions: Vec<_> = self
            .state
            .session
            .open
            .iter()
            .map(|o| o.session.id.clone())
            .collect();
        for session in sessions {
            let end = Command::Agent(agent::Command::End { session });
            dispatch(end, &mut self.state, &self.services, &self.spawner);
        }
    }

    /// Where the drag left the boundaries, for the next run.
    fn keep_panes(&mut self) {
        let path = groove_config::panes::path(&self.state.env.data_dir);
        if let Err(e) = groove_config::panes::save(&path, &self.ui.split.panes()) {
            self.state.failed(e.into());
        }
    }

    fn apply(&mut self, event: Event) {
        apply(event, &mut self.state);
        self.redraw();
    }
}

/// The narrowest the window may be.
pub(super) const MIN_WIDTH: f64 = 960.0;
pub(super) const MIN_HEIGHT: f64 = 600.0;

pub(super) fn size_of(window: &Window) -> Size {
    let size = window.inner_size();
    Size::new(size.width, size.height)
}

fn icon_of(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::ColResize => CursorIcon::ColResize,
        Cursor::RowResize => CursorIcon::RowResize,
        Cursor::Text => CursorIcon::Text,
    }
}
