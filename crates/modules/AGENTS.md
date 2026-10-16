# modules — one concern each

A module does one thing on the base crates and `groove-types`, and may call another
module. It knows no service and no user action. Its IO is an `async fn` and only that,
so it runs in a controller's job on the tokio pool; a blocking path would stall the
main thread that owns `AppState`.

A module returns `groove_types::Error` unless a caller matches one of its variants, or
its variants map to different kinds; only then does it keep its own error enum.

Test each module against the base: `forge` and `provider` on a `wiremock` server,
`terminal` by asserting the grid from a byte stream, `text` pure, `git` on temp repos
set up through `sh` in `crates/modules/git/src/tests/fixture.rs`: it passes the
identity and turns signing off, so a scene commits on any machine.
