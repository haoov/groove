# Glossary

One line a term. The other docs link here at a term's first use.

## The work

### Session
One unit of work in the window: its repos, worktrees, agent, terminals and log. Its kind is task, explorer, review or routine.

### Task
A session that works a ticket from a provider. It has a branch and usually ends in an MR.

### Explorer
A session with no ticket yet. Filing a ticket and adopting it turns it into a task.

### Review
A session on someone else's MR, checked out to read and annotate.

### Routine session
The session a standalone routine runs in. It holds only its agent, and never shows on the board.

### Worktree
A git worktree cut from a repo's clone for one session, at `<root>/worktrees/<session>/<project>/<branch>`.

### Clone
The repo's own checkout under `<root>/main`. Worktrees are cut from it; nothing writes to it.

### Provider
Where tasks come from: GitHub issues or a Notion database.

### Forge
Where MRs live: GitLab or GitHub. Its host decides which one.

## The app

### Capability
One of the five top-level parts of Groove: Tasks, Agent, Sessions, Workspace, Config.

### Tool set
A feature of a capability the user can name, such as Diff or Routines. Never a surface, never a module.

### Surface
A region the user sees: the board, the rail, the session, the review sheet, settings.

### Ask
A write from the agent that waits for the user to approve, review or refuse it.

### Auto-approve
A per-session switch: when it is on, an ask is approved as it arrives.

### Feed
The rail's log of what the open sessions did, newest first.

## The code

### Layer
One row of the workspace's dependency order: base, shared, modules, services, controllers, ui. See [architecture.md](architecture.md#layers).

### Service
The crate that owns one part of the app: its slice of `AppState` and the operations on it.

### Controller
The module that holds one function per user action and calls the services in order.

### Command
One variant per controller function. Input, the palette, a keybinding and an agent tool all produce one.

### `dispatch`
The one match that runs a `Command`: the sync part now, the async part through the `Spawner`.

### Job
The async part of a controller function, run on the tokio pool. Its output is a continuation.

### Continuation
A closure a job returns, run on the main thread, which writes the job's result into `AppState`.

### Generation
The number a read carries so its continuation can tell a newer read started. A stale answer is dropped.

### Event
Input from the outside world that no command asked for, applied by `apply`: a hook, a file change, window focus.

### Frame
The display list `ui` draws from `&AppState` each frame, and `gfx` renders.
