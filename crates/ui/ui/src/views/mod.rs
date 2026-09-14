//! The surfaces. A view reads `AppState` and composes widgets; it never calls a service.
//! Every surface is a directory: its own file, then its components.

pub mod session;
pub mod shared;
