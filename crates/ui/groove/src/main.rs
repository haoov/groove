//! Groove. One window, one event loop, one writer of `AppState`.
//! `groove --explore` opens an explorer at start, until the board exists.

mod app;
mod keys;

use std::path::PathBuf;
use std::sync::Arc;

use groove_controllers::{Continuation, Deliver, Env, Event, Services, TokioSpawner};
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
    let config_state = groove_controllers::config_service::State { config };
    let root = config_state.worktree_root(&env.home);
    let services = runtime.block_on(services(&env, &root))?;
    let config = config_state.config;
    let mut app = app::App::new(spawner, services, env, config, explore);
    event_loop.run_app(&mut app)?;
    let result = app.into_result();
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    result
}

/// Every module on the one database, handed to its service.
async fn services(
    env: &Env,
    root: &std::path::Path,
) -> Result<Services, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&env.data_dir)?;
    let db = groove_db::Db::open(&env.data_dir.join("app.db")).await?;
    let store = groove_sessions::Store::new(db.clone());
    let pool = groove_worktree::Pool::new(db, root);
    Ok(Services {
        session: groove_controllers::session_service::Service::new(store, pool),
    })
}

/// `$HOME` and the XDG dirs under `groove`; the legacy app keeps `com.haoov.groove`.
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
        config_dir: config.join("groove"),
        data_dir: data.join("groove"),
        home,
        plugin_dirs: Vec::new(),
    }
}
