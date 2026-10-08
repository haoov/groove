//! The event loop's own half of `App`: what winit hands it, and what it hands back.

use std::sync::Arc;
use std::time::{Duration, Instant};

use groove_controllers::{Command, Event, dispatch, session, task};
use groove_gfx::{Fonts, Renderer};
use groove_ui::input::Input;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowAttributes, WindowId};

use super::{App, MIN_HEIGHT, MIN_WIDTH, START_HEIGHT, START_WIDTH, size_of};
use crate::Message;
use crate::keys::input_of;
use groove_controllers::Window as WindowFocus;
use groove_ui::input::Delta;

/// The name the desktop matches the window to its launcher by, `groove` on X11 and Wayland.
#[cfg(target_os = "linux")]
fn named(attributes: WindowAttributes) -> WindowAttributes {
    use winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
    let attributes = WindowAttributesExtWayland::with_name(attributes, "groove", "groove");
    WindowAttributesExtX11::with_name(attributes, "groove", "groove")
}

#[cfg(not(target_os = "linux"))]
fn named(attributes: WindowAttributes) -> WindowAttributes {
    attributes
}

impl ApplicationHandler<Message> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        let attributes = Window::default_attributes()
            .with_title("Groove")
            .with_inner_size(winit::dpi::LogicalSize::new(START_WIDTH, START_HEIGHT))
            .with_min_inner_size(winit::dpi::LogicalSize::new(MIN_WIDTH, MIN_HEIGHT));
        let attributes = named(attributes);
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
        for command in [
            Command::Session(session::Command::Restore),
            Command::Session(session::Command::List),
            Command::Task(task::Command::Load),
            Command::Delivery(groove_controllers::delivery::Command::ReviewQueue),
            Command::Agent(groove_controllers::agent::Command::ListSkills),
        ] {
            dispatch(command, &mut self.state, &self.services, &self.spawner);
        }
        if std::mem::take(&mut self.explore) {
            let open = Command::Session(session::Command::OpenExplorer { title: None });
            dispatch(open, &mut self.state, &self.services, &self.spawner);
        }
    }

    /// Nothing moving, nothing to draw: the loop sleeps until an event or the next tick.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.clock();
        let Some(after) = self.pace() else {
            self.tick = None;
            return event_loop.set_control_flow(ControlFlow::Wait);
        };
        let now = Instant::now();
        let due = match self.tick {
            Some(at) if at > now => at.min(now + after),
            _ => {
                self.redraw();
                now + after
            }
        };
        self.tick = Some(due);
        event_loop.set_control_flow(ControlFlow::WaitUntil(due));
    }

    fn user_event(&mut self, _: &ActiveEventLoop, message: Message) {
        match message {
            Message::Continue(continuation) => {
                continuation(&mut self.state, &self.services, &self.spawner);
                self.watch();
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
            WindowEvent::Focused(focused) => self.apply(Event::Window(WindowFocus::Focus(focused))),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ModifiersChanged(mods) => self.modifiers = mods.state(),
            WindowEvent::KeyboardInput {
                is_synthetic: true, ..
            } => {}
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(input) = self
                    .pasting(&event)
                    .or_else(|| input_of(&event, self.modifiers))
                {
                    self.input(input);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.moved_to((position.x as f32, position.y as f32))
            }
            WindowEvent::CursorLeft { .. } => {
                if self.ui.hover.take().is_some() {
                    self.redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = self.cursor;
                let delta = delta_of(delta, self.modifiers.shift_key());
                self.input(Input::Scroll { x, y, delta });
            }
            WindowEvent::MouseInput { state, button, .. } => self.button(state, button),
            _ => {}
        }
    }
}

impl App {
    /// A chord the keymap binds to a paste, with what the clipboard holds in it.
    fn pasting(&self, event: &winit::event::KeyEvent) -> Option<Input> {
        let bound = match input_of(event, self.modifiers) {
            Some(Input::Key { key, mods }) => {
                groove_ui::keymap::pastes(self.state.config.config.as_ref(), key, mods)
            }
            _ => false,
        };
        let inserted = self.modifiers.shift_key()
            && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Insert);
        if event.state != ElementState::Pressed || !(bound || inserted) {
            return None;
        }
        Some(Input::Paste(self.services.clipboard.read()?))
    }

    /// The pointer moved: what is under it, and the drag or the selection it carries.
    fn moved_to(&mut self, at: (f32, f32)) {
        self.cursor = at;
        self.point();
        let (x, y) = at;
        let moved = groove_ui::input::hover(&mut self.ui, &self.hits, x, y);
        if self.ui.pointing() {
            self.input(Input::Move { x, y });
        } else if moved {
            self.redraw();
        }
    }

    /// A mouse button down or up. A drag that ends keeps where it left the boundary.
    fn button(&mut self, state: ElementState, button: MouseButton) {
        let (x, y) = self.cursor;
        match (state, button) {
            (ElementState::Pressed, MouseButton::Left) => {
                let mods = crate::keys::mods_of(self.modifiers);
                self.input(Input::Press { x, y, mods })
            }
            (ElementState::Pressed, MouseButton::Right) => self.input(Input::Menu { x, y }),
            (ElementState::Pressed, MouseButton::Middle) => self.input(Input::Middle { x, y }),
            (ElementState::Released, MouseButton::Left) => {
                let dragged = self.ui.dragging();
                self.input(Input::Release);
                if dragged {
                    self.keep_panes();
                }
            }
            _ => {}
        }
    }
}

/// A wheel notch is lines; a trackpad is pixels, and up is away from the user.
/// With shift held, a wheel that only goes down goes across.
fn delta_of(delta: MouseScrollDelta, shift: bool) -> Delta {
    match delta {
        MouseScrollDelta::LineDelta(0.0, down) if shift => Delta::Lines {
            across: down,
            down: 0.0,
        },
        MouseScrollDelta::LineDelta(across, down) => Delta::Lines { across, down },
        MouseScrollDelta::PixelDelta(at) => Delta::Pixels {
            across: at.x as f32,
            down: at.y as f32,
        },
    }
}

/// How often the agents are refitted while a split is dragged.
pub(super) const FIT_MS: u64 = 100;

/// How often an idle window redraws its clocks.
pub(super) const CLOCK_S: u64 = 15;

/// How long until the second `due` begins.
pub(super) fn until(due: groove_types::Timestamp) -> Duration {
    let at = std::time::UNIX_EPOCH + Duration::from_secs(due.seconds().max(0) as u64);
    at.duration_since(std::time::SystemTime::now())
        .unwrap_or_default()
}
