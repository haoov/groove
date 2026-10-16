# ui — present, translate input

Three crates, top down: `groove` is the binary (the winit loop, the one writer of
`AppState`), `ui` holds the views and turns input into `Command`s, `kit` holds the
context, tokens, styles, layout and widgets on plain data. A view draws `&AppState` into
a `Frame`; it calls no service and decides no order, it sends a `Command`.

## Writing a view

- A view lays out and draws through the kit: boxes and text leaves, no rect cut by
  hand, no Taffy, no drawing primitive. Enforced by `a_view_lays_out_through_the_kit`,
  `only_the_kit_depends_on_taffy` and `a_view_draws_through_the_context`.
- A size is a token in `crates/ui/kit/src/base/tokens.rs`, a text style a function in
  `crates/ui/kit/src/base/style.rs`. Enforced by `every_size_comes_from_the_tokens` and
  `every_style_comes_from_one_file`.
- A part only one view draws stays in that view, built from kit widgets; a widget joins
  the kit once a second view needs it.
- A view file draws one region. Over the ceiling it becomes a directory named after the
  region, one file a part.
- Ui state sits with its view; `Ui` keeps beside them only what crosses views.

`docs/design.md` holds what each surface shows and the rules it follows; read it before
changing a surface.

## Tests

Assert the `Frame` against a fixture `AppState`, with no GPU: `app`, `render`, `press`
and `click` in `crates/ui/ui/src/tests.rs` build the state, draw it and drive input.
`crates/ui/ui/src/tests/structure.rs` holds the rules above.
