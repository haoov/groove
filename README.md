# Groove

A desktop workspace for task-driven development. One window holds the task, its
repos, its branches, its diff, its merge request and an agent that can act on all of
them — so a piece of work stays in one place instead of a browser, a terminal and an
editor that know nothing about each other.

Groove reads your work from **Notion** or **GitHub** and keeps a pool of git
worktrees on disk. Opening a task checks out a branch in every repo it touches;
finishing it puts the worktrees away.

<!-- SCREENSHOT: full window on a task session — editor + diff + agent console.
     Landscape, ~1600px. The one that has to sell the idea. -->
![A task session](docs/images/workspace.png)

*A session: the task's repos and worktrees, its diff against the merge base, and an
agent that can see all of it.*

---

<!-- SCREENSHOT: Home, Live tab, a session expanded, filter bar in use. -->
![Home](docs/images/home.png)

*Home — what is checked out, what is queued, and the reviews waiting on you. The
filter takes `field:value`: `provider:github`, `priority:high`, `-kind:explorer`.*

<!-- SCREENSHOT: agent console mid-turn with an approval dialog up. -->
![Agent and approvals](docs/images/agent.png)

*The agent runs Claude Code inside the session and reaches the workspace over MCP.
Anything that leaves the machine or rewrites history stops for approval first.*

---

## What it gives you

- **Tasks become worktrees.** A task from Notion or GitHub Issues provisions a branch
  per repo, and every panel from then on is scoped to it.
- **Diffs that tell the truth.** Compared against the merge base, so you see what
  your branch introduced — not what upstream did meanwhile.
- **Merge requests in place.** Create, approve, and answer review threads inline, on
  GitLab and GitHub alike.
- **An agent that shares the workspace,** not a chat window beside it — with every
  outward action gated by an approval.
- **Notes on any line,** whether or not an MR exists yet.

## Install

Grab a build from [Releases](https://github.com/haoov/groove/releases) — `.deb`,
`.rpm` or `.AppImage` for Linux, `.dmg` for macOS.

From source:

```sh
pnpm install
pnpm tauri dev          # or: pnpm tauri build
```

Groove drives `git`, `claude`, and `glab` / `gh` — it holds no tokens of its own. It
checks what is present on first run and tells you what is missing.

## Contributing

The module map and the invariants worth knowing before changing anything:
**[CLAUDE.md](CLAUDE.md)**.

```sh
pnpm test  ·  pnpm lint  ·  npx tsc --noEmit  ·  cd src-tauri && cargo test --lib
```
