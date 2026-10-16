# docs — the rules over the code

`README.md` maps which file answers which question; put a new rule in the file it names.

A rule is a bold sentence. In `architecture.md` it is followed by `*Example:*` with a
real case from the code, then the sentence naming the test that holds it; a rule no test
can check says so.

- A term is defined once, one line in `glossary.md`; a doc links it at its first use.
- A cited path starts at the repo root, as `crates/<layer>/<crate>`, so the drift test
  can check it.
- The voice is short declarative sentences, present tense, about what Groove does.
