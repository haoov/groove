# controllers — one function per user action

A controller function is one action, a read included: which services, in what order,
what to undo when a later step fails. It is the only place two services meet. It holds
no state of its own and reaches modules only through a service.

## Adding an action

1. A variant in the controller's `Command`, its doc opening on the id:
   `` /// `cluster.usage`: a pod's usage, read once from metrics-server. ``
2. Its arm in `Command::id` and in `dispatch`. The id `controller.function` is the
   palette entry, the agent's tool name and the timeline label.
3. The sync part changes `AppState` now. The IO goes in a `Job` through the `Spawner`
   and ends in a `Continuation` that writes the result back. Read what the job needs
   inside the job: `dispatch`, `apply` and a continuation run on the main thread.
4. A read that a newer read can overtake carries a generation, as `load` in
   `crates/controllers/controllers/src/workspace/diff.rs` does.

A controller that grows splits into a directory beside its file; `Command` and
`dispatch` stay in the file, so its actions read in one place.

## Tests

`fresh` in `crates/controllers/controllers/src/tests/fixture.rs` gives a temp home, a
`SyncSpawner`, the `Services` and an `AppState`. Dispatch, `drain` the spawner to apply
the continuations, then assert `AppState`.

`crates/controllers/controllers/src/tests/layers.rs` and
`crates/controllers/controllers/src/tests/docs.rs` hold the whole workspace to its
layers, its ceilings and its docs.
