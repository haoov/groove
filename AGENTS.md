# Groove — working in this repo

Groove is a Rust desktop app drawn on the GPU.

`docs/` holds the intent, the rules and the decisions; the code holds what exists. Start
at `docs/README.md`. A doc never lists what the code already lists: a crate's role is its
`Cargo.toml` `description`, an action is a `Command` variant, a plan is a GitHub issue.

When a change breaks a rule in `docs/`, change the rule in the same MR or change the code.
`crates/controllers/controllers/src/tests/docs.rs` fails on a broken link, a cited path
that no longer exists, a crate with no description, or a rule naming a missing test.
