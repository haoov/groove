# Groove — working in this repo

Groove is a Rust desktop app drawn on the GPU: a winit window, a wgpu renderer, no web
view. A crate lives at `crates/<layer>/<crate>`; the `AGENTS.md` of each layer holds
what to know before writing in it.

`docs/` holds the intent, the rules and the decisions; the code holds what exists. Read
`docs/README.md` before adding a feature or a surface, `docs/architecture.md` before
adding a crate or calling across a layer. A doc never lists what the code already lists:
a crate's role is its `Cargo.toml` `description`, an action is a `Command` variant, a
plan is a GitHub issue.

When a change breaks a rule in `docs/`, change the rule in the same MR or change the code.
`crates/controllers/controllers/src/tests/docs.rs` holds `docs/` and every `AGENTS.md` to
the code: it fails on a broken link, a cited path that no longer exists, a crate with no
description, or a rule naming a missing test.

## Done

A change is done when it passes what CI runs:

- `cargo fmt --all --check`
- `cargo lint`, clippy with `-D warnings`
- `cargo test --workspace --locked`

Every file stands under 300 lines and every function under 50; a test file under 500 and
a test under 100. Enforced by `no_file_stands_above_its_ceiling` and
`no_function_stands_above_its_ceiling`.

CI skips a change that touches only Markdown: after editing a doc or an `AGENTS.md`, run
`cargo test -p groove-controllers docs` yourself.

The MR title is a Conventional Commit: it becomes the squashed commit and its changelog
line.
