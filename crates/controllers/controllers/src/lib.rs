//! The controllers are the API. Commands go down as data, results come back as
//! continuations, the outside world arrives as events. One thread applies all three.

pub mod agent;
pub mod asker;
pub mod config;
pub mod delivery;
pub mod session;
pub mod task;
pub mod tools;
pub mod workspace;

mod command;
mod event;
mod services;
mod spawn;
mod state;
mod timeline;

#[cfg(test)]
mod tests;

pub use command::{Command, dispatch};
pub use event::{Event, Window, apply};
pub use services::Services;
pub use spawn::{Continuation, Deliver, Spawner, SyncSpawner, TokioSpawner};
pub use state::{AppState, Env, Pending, Told};

pub use groove_agent_service as agent_service;
pub use groove_config_service as config_service;
pub use groove_delivery_service as delivery_service;
pub use groove_session_service as session_service;
pub use groove_task_service as task_service;
pub use groove_workspace_service as workspace_service;
