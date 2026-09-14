use std::sync::Arc;
use std::time::{Duration, Instant};

use groove_controllers::{
    AppState, Command, Env, Event, Services, TokioSpawner, Window as WindowEvent_, agent, apply,
    dispatch, session,
};
use groove_gfx::{Fonts, Renderer, Size};
use groove_types::Config;
use groove_ui::{Metrics, Ui};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

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
        for command in groove_ui::layout_commands(&self.state, metrics) {
            dispatch(command, &mut self.state, &self.services, &self.spawner);
        }
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let frame = groove_ui::view(&self.state, &self.ui, metrics, renderer.fonts());
        let _ = renderer.render(&frame);
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
            cell: renderer.fonts().cell_size(tokens.mono),
            tick: self.started.elapsed().as_millis() as u64,
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
            .with_inner_size(winit::dpi::LogicalSize::new(1440.0, 900.0));
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

    /// While a job is in flight the status line moves: one frame every 120 ms.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.pending.is_empty() {
            event_loop.set_control_flow(ControlFlow::Wait);
        } else {
            self.redraw();
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(120),
            ));
        }
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
                let Some(input) = input_of(&event, self.modifiers) else {
                    return;
                };
                for command in groove_ui::input::handle(input, &mut self.ui, &self.state) {
                    dispatch(command, &mut self.state, &self.services, &self.spawner);
                }
                self.redraw();
            }
            _ => {}
        }
    }
}

fn size_of(window: &Window) -> Size {
    let size = window.inner_size();
    Size::new(size.width, size.height)
}
