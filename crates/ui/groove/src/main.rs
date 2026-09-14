//! Groove. One window, one event loop, one writer of `AppState`.
//! `groove --explore` opens an explorer at start, until the board exists.

mod app;
mod keys;

use std::path::PathBuf;
use std::sync::Arc;

use groove_controllers::{Continuation, Deliver, Env, Event, TokioSpawner};
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
    let explore = std::env::args().any(|a| a == "--explore");
    let env = env();
    let config = groove_controllers::config_service::load(&env.config_dir)?;
    let mut app = app::App::new(spawner, env, config, explore);
    event_loop.run_app(&mut app)?;
    app.into_result()
}

/// `$HOME` and the XDG data dir under the bundle identifier.
fn env() -> Env {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    Env {
        config_dir: config.join("com.haoov.groove"),
        data_dir: data.join("com.haoov.groove"),
        home,
        plugin_dirs: Vec::new(),
    }
}
