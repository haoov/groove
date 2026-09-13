//! Groove. One window, one event loop, one writer of `AppState`.

mod app;
mod keys;

use std::sync::Arc;

use groove_controllers::{Continuation, Deliver, Event, TokioSpawner};
use winit::event_loop::{EventLoop, EventLoopProxy};

/// What crosses from the pool and the outside world into the loop.
pub enum Message {
    Continue(Continuation),
    Event(Event),
}

/// winit's proxy as the continuation sink.
struct Proxy(EventLoopProxy<Message>);

impl Deliver for Proxy {
    fn deliver(&self, continuation: Continuation) {
        let _ = self.0.send_event(Message::Continue(continuation));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    let event_loop = EventLoop::<Message>::with_user_event().build()?;
    let spawner = TokioSpawner::new(
        runtime.handle().clone(),
        Arc::new(Proxy(event_loop.create_proxy())),
    );
    let mut app = app::App::new(spawner);
    event_loop.run_app(&mut app)?;
    app.into_result()
}
