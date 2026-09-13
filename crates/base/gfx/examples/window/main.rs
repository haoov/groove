//! Opens a window and draws the demo frame. `cargo run -p groove-gfx --example window`.
//! `p` toggles the palette layer, Escape quits. `GROOVE_DEMO_PALETTE=1` opens with it.

mod demo;

use std::sync::Arc;

use groove_gfx::{Fonts, Renderer, Size};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    palette: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("groove gfx")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let Ok(window) = event_loop.create_window(attributes) else {
            event_loop.exit();
            return;
        };
        let window = Arc::new(window);
        match Renderer::windowed(window.clone(), size_of(&window), Fonts::new()) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(_) => event_loop.exit(),
        }
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::KeyboardInput { event, .. } => self.key(event_loop, event),
            _ => {}
        }
    }
}

impl App {
    fn key(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        match event.logical_key {
            Key::Named(NamedKey::Escape) => event_loop.exit(),
            Key::Character(c) if c == "p" => {
                self.palette = !self.palette;
                self.redraw();
            }
            _ => {}
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
        let frame = demo::frame(
            renderer,
            size_of(window),
            window.scale_factor() as f32,
            self.palette,
        );
        let _ = renderer.render(&frame);
    }
}

fn size_of(window: &Window) -> Size {
    let size = window.inner_size();
    Size::new(size.width, size.height)
}

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        palette: std::env::var_os("GROOVE_DEMO_PALETTE").is_some(),
        ..App::default()
    };
    event_loop.run_app(&mut app)
}
