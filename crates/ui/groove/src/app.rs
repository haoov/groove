use std::sync::Arc;

use groove_controllers::{AppState, Event, TokioSpawner, Window as WindowEvent_, apply, dispatch};
use groove_gfx::{Fonts, Renderer, Size};
use groove_ui::Ui;
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
    modifiers: ModifiersState,
    failure: Option<groove_gfx::Error>,
}

impl App {
    pub fn new(spawner: TokioSpawner) -> Self {
        Self {
            window: None,
            renderer: None,
            state: AppState::default(),
            ui: Ui::default(),
            spawner,
            modifiers: ModifiersState::empty(),
            failure: None,
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

    fn draw(&mut self) {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return;
        };
        let frame = groove_ui::view(
            &self.state,
            &self.ui,
            size_of(window),
            window.scale_factor() as f32,
        );
        let _ = renderer.render(&frame);
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
    }

    fn user_event(&mut self, _: &ActiveEventLoop, message: Message) {
        match message {
            Message::Continue(continuation) => {
                continuation(&mut self.state);
                self.redraw();
            }
            Message::Event(event) => self.apply(event),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => self.redraw(),
            WindowEvent::Focused(focused) => {
                self.apply(Event::Window(WindowEvent_::Focus(focused)))
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ModifiersChanged(mods) => self.modifiers = mods.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                let Some(input) = input_of(&event, self.modifiers) else {
                    return;
                };
                if let Some(command) = groove_ui::input::handle(input, &mut self.ui, &self.state) {
                    dispatch(command, &mut self.state, &self.spawner);
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
