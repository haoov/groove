pub(crate) mod components;

/// The surface's own file carries its name.
#[allow(clippy::module_inception)]
mod session;

pub(crate) use components::files::changed;
pub use session::{SessionUi, Tab, draw};
