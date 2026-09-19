//! The surfaces. A view reads `AppState` and composes widgets; it never calls a service.
//! Every surface is a directory: its own file, then the regions it draws.

pub mod board;
pub mod session;
pub mod shared;
