use std::sync::Arc;
use std::time::{Duration, Instant};

use groove_controllers::{
    AppState, Command, Env, Event, Services, TokioSpawner, Window as WindowEvent_, agent, apply,
    dispatch, session,
};
use groove_gfx::{Fonts, Renderer, Size};
use groove_types::{AttentionClass, Config, Timestamp};
use groove_ui::input::{Delta, Input};
use groove_ui::{Cursor, Hits, Metrics, Ui};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{CursorIcon, Window, WindowId};

use crate::Message;
use crate::keys::input_of;

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
    /// When the agents were last fitted to their pane, to throttle a drag.
    fitted: Instant,
    failure: Option<groove_gfx::Error>,
    explore: bool,
    started: Instant,
}

impl App {
    pub fn new(
        spawner: TokioSpawner,
        services: Services,
        env: Env,
        config: Option<Config>,
        explore: bool,
    ) -> Self {
        let mut state = AppState::new(env);
        state.config.config = config;
        Self {
            window: None,
            renderer: None,
            state,
            ui: Ui::default(),
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
        self.point();
    }

    /// How long the window may sleep: a frame while a mark turns, otherwise long
    /// enough that a row's `2m` becomes `3m` on its own.
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
        let waiting = !self.state.agent.agents.is_empty();
        waiting.then(|| Duration::from_secs(CLOCK_S))
    }

    /// Every agent's grid to its pane. A drag is throttled: one resize repaints the
    /// whole TUI, and a grid only changes by whole cells anyway.
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
        let commands =
            groove_ui::input::handle(input, &mut self.ui, &self.state, &self.hits, metrics);
        for command in commands {
            dispatch(command, &mut self.state, &self.services, &self.spawner);
        }
        self.redraw();
    }

    fn metrics(&mut self) -> Option<Metrics> {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return None;
        };
        let scale = window.scale_factor() as f32;
        let tokens = groove_ui::Tokens::new(scale);
        Some(Metrics {
            size: size_of(window),
            scale,
            cell: renderer.fonts().cell_size(tokens.code),
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

    fn apply(&mut self, event: Event) {
        apply(event, &mut self.state);
        self.redraw();
    }
}

impl ApplicationHandler<Message> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        let attributes = Window::default_attributes()
            .with_title("Groove")
            .with_inner_size(winit::dpi::LogicalSize::new(1440.0, 900.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(MIN_WIDTH, MIN_HEIGHT));
        let Ok(window) = event_loop.create_window(attributes) else {
            event_loop.exit();
            return;
        };
        let window = Arc::new(window);
        match Renderer::windowed(window.clone(), size_of(&window), Fonts::new()) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => {
                self.failure = Some(e);
                event_loop.exit();
            }
        }
        self.window = Some(window);
        dispatch(
            Command::Session(session::Command::Restore),
            &mut self.state,
            &self.services,
            &self.spawner,
        );
        if std::mem::take(&mut self.explore) {
            let open = Command::Session(session::Command::OpenExplorer { title: None });
            dispatch(open, &mut self.state, &self.services, &self.spawner);
        }
    }

    /// Nothing moving, nothing to draw: the loop sleeps until an event.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(after) = self.pace() else {
            return event_loop.set_control_flow(ControlFlow::Wait);
        };
        self.redraw();
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + after));
    }

    fn user_event(&mut self, _: &ActiveEventLoop, message: Message) {
        match message {
            Message::Continue(continuation) => {
                continuation(&mut self.state, &self.services, &self.spawner);
                self.redraw();
            }
            Message::Event(event) => self.apply(event),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.end_agents();
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.redraw(),
            WindowEvent::Focused(focused) => {
                self.apply(Event::Window(WindowEvent_::Focus(focused)))
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ModifiersChanged(mods) => self.modifiers = mods.state(),
            WindowEvent::KeyboardInput {
                is_synthetic: true, ..
            } => {}
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(input) = input_of(&event, self.modifiers) {
                    self.input(input);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                self.point();
                let (x, y) = self.cursor;
                let moved = groove_ui::input::hover(&mut self.ui, &self.hits, x, y);
                if self.ui.dragging() {
                    self.input(Input::Move { x, y });
                } else if moved {
                    self.redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if self.ui.hover.take().is_some() {
                    self.redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = self.cursor;
                self.input(Input::Scroll {
                    x,
                    y,
                    delta: delta_of(delta),
                });
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.cursor;
                self.input(Input::Press { x, y });
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.input(Input::Release),
            _ => {}
        }
    }
}

/// A wheel notch is lines; a trackpad is pixels, and up is away from the user.
fn delta_of(delta: MouseScrollDelta) -> Delta {
    match delta {
        MouseScrollDelta::LineDelta(_, lines) => Delta::Lines(lines),
        MouseScrollDelta::PixelDelta(position) => Delta::Pixels(position.y as f32),
    }
}

/// How often the agents are refitted while a split is dragged.
const FIT_MS: u64 = 100;

/// The narrowest the window may be: every column at its minimum, and room to read.
const MIN_WIDTH: f64 = 960.0;
const MIN_HEIGHT: f64 = 600.0;

/// One frame of a turning mark, and how often an idle window redraws its clocks.
const FRAME_MS: u64 = 120;
const CLOCK_S: u64 = 15;

fn icon_of(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::ColResize => CursorIcon::ColResize,
    }
}

fn size_of(window: &Window) -> Size {
    let size = window.inner_size();
    Size::new(size.width, size.height)
}
