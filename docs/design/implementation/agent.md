# Agent — implementation

How the tool sets in [../capabilities/agent.md](../capabilities/agent.md) are made:
today, from the code; planned, in the words of [../architecture.md](../architecture.md).

## Multi-agent

### Today

One Claude Code process per session in a PTY at the worktree root. The Claude session
id is a v5 UUID of the task id: `--session-id` on the first launch, `--resume` after.
Reload stops the PTY and starts it again with `--resume`. Exit is detected through
`pty_exit`. The agents sidebar lists every session's agent with idle, working or
waiting; a pop-out window mirrors the console over a bridge.

Approvals: every outward write posts a request to the bridge (`approvals/bridge.rs`),
which stores it in SQLite and blocks the caller until `resolve`. Pending requests are
surfaced again at startup. A `git.commit` from the agent is stamped `index_only`. The
per-session `autoApprove` flag is frontend state: a request from a session with it on
is approved as it arrives, then a notification is shown.

### Planned

**Service `agent`.** Its slice of `AppState`: one `SessionActivity` per open session —
status, the pending asks, the auto-approve flag, the time of the last change, the last
seen mark.

**Status**, derived in the service from hooks, `pty_exit` and lifecycle events:

| Status | From | Until |
|---|---|---|
| working | `PreToolUse`, `UserPromptSubmit` | `Stop` |
| done, unseen | `Stop`, `Notification` | the session is selected in the rail |
| idle | at start; done and seen | next prompt |
| exited | `pty_exit` non-zero | reopen |
| error | a failed controller on the session | the next successful one |

The done row's text comes from `git` since the last seen mark: commits and files.

**Asks.** With auto-approve off, every write the agent sends through `tools` becomes
an ask on the row: the op and its subject, Approve and Review. With it on, `approvals`
approves on arrival and records the write; the timeline gets the entry. The flag is
per session, held in the slice; a new session takes the default from Config ›
Preferences.

**Attention class**, computed in the service: needs you — ask, exited, error; act
when you look — done unseen; moving — working; quiet — idle. Colour and
motion follow [../design.md](../design.md).

**Controllers**, in `agent`:

| id | does |
|---|---|
| `agent.start` | called by `session.open`: `agent-launch` builds the flags and spawns through `terminal` |
| `agent.end` | called by `session.close`: ends the PTY, cancels the session's token |
| `agent.reload` | end, then start with `--resume` |
| `agent.approve` · `agent.refuse` | `approvals.resolve`, then the row and the timeline |
| `agent.review` | opens the review sheet in ui state |
| `agent.auto_approve` | flips the session's flag |
| `agent.get_activity` · `agent.get_asks` | reads: the rail's rows |
| `agent.resize` | the pane's size to the PTY |

**Rail** reads the slice and nothing else; the feed reads `timeline` for the opened
sessions.

**Removed.** The agents sidebar, the pop-out window and its bridge, dialogs and
notifications as surfaces, the frontend `autoApprove` short-circuit.

## MCP tools

### Today

An axum server on `127.0.0.1:27413` with `/sse` and `/message`, one per-launch bearer
token, the session id in the SSE URL. 35 tools: 16 reads, 16 writes through the
bridge, 3 local annotation writes. `tools/definitions.rs` holds every description —
the one place that tells the agent how to write a commit message, MR text, an
annotation or a task body. `tools/args.rs` types every argument. A write on a worktree
op (`WORKTREE_OPS`) is bound to a worktree the session owns; a foreign worktree is
refused. Launch files — `mcp.json`, `hooks.curl`, `settings.json`, `prompt.md` — are
written per task with mode 0600.

### Planned

**Module `tools`**: the definitions, the argument types, the harness descriptions.
Each tool names the controller function it calls; a test asserts the map is total.

**`mcp-server`**, ui layer: axum as today; a tool call becomes a `Command` and goes
through `dispatch` like a click. The response is the controller's result. Writes pass
`approvals` inside the controller, so the blocking-until-resolved contract stays.

**Scoped to the session**: the SSE URL still carries the session id, the token still
comes from the launch. Every controller called through the server gets that session
as its default.

**Harness description**: kept as the single source. The core prompt and the skills
point to the tools; they never restate how to write.

## Activity

### Today

Six hooks in `settings.json` — SessionStart, UserPromptSubmit, PreToolUse,
PostToolUse, Notification, Stop — each a `curl -K hooks.curl -X POST` to
`hook_url(task_id)`. `agent_hooks` keeps `AgentActivity { state, tool }` per session
in a shared map; edit tools carry `file_path`, Bash carries the command. File-editing
tools and `Stop` drive the session refresh. The frontend reads `get_agent_activity`
once and then the `agent_activity` event.

### Planned

**Module `hooks`**: the receiver, mounted on `mcp-server`. Each POST becomes
`Event::Agent(Hook { session, kind, tool })` on the proxy. Bursts on file-editing
tools are coalesced before they reach the loop.

**Module `activity`**: the fold from hook events to status, used by the `agent`
service's `apply`.

**Module `timeline`**: one table — session, time, kind, subject, payload. Written by
`apply` for hook kinds worth keeping (turn start, turn end, tool with a file), by
`approvals` for every write, by `forge` for MR and CI events, by `annotations` for
notes. Read by the rail's feed and the session's timeline.

## Core prompt

### Today

`skills/prompt.md` rendered by `skills::core_prompt(task_id, session)` and passed with
`--append-system-prompt-file` at every launch. Interpolates stable identity only: the
session. Rules: the session is fixed, a write waits for a human, worktree paths are
`<project>/<branch>`, never write in a clone.

### Planned

**Module `agent-launch`**: the template and the render, unchanged in content except
the approval rule, which becomes "a write may wait for a human". Passed per launch,
never persisted into the conversation.

## Skills

### Today

Eight core skills as `SKILL.md` under `skills/core/`, listed in `CORE_SKILLS`. User
skills under `<config>/user-skills/skills/`, written by Settings directly or by the
agent through `save_user_skill`, the one local write that goes through the bridge.
`plugin_dirs()` gives every dir passed as `--plugin-dir`. A write marks skills stale;
the console offers a reload. The console's Actions menu lists `list_agent_skills` and
sends `/groove:<name>`.

### Planned

**Module `skills`**: the two directories, the list with hints, read, save, delete,
the stale flag. Controllers `agent.list_skills`, `agent.read_skill`, `agent.save_skill`,
`agent.delete_skill`, `agent.send_skill`. `agent.save_skill` from the agent asks like
any write.

**Buttons.** A worktree row's delivery state names its skill; the button sends
`agent.send_skill` with it. The agent pane keeps one menu for the rest.

## Needs

- [ ] `SessionActivity` in `types`: status, asks, auto-approve, last change, last seen.
- [ ] Timeline table and its migration.
- [ ] Hook coalescing window.
