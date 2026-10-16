# services — one part of the app each

A service owns one slice of `AppState` and the operations on it. Its crate's `//!` doc
says what it owns.

- `State` is the slice: data on the main thread, changed by sync methods.
- Its IO is `async fn`s a controller spawns as a job. A `Service` struct, where there is
  one, holds the module handles, cheap to clone into that job.

A service depends on modules and `groove-types` only. Work that spans two services is a
controller function in `crates/controllers/controllers`. Enforced by
`no_service_depends_on_a_service`.

A service that grows splits into files, one a tool set, as `crates/services/workspace`
does. Test each service in isolation, on fakes of its modules.
