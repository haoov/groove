# Agent — implementation

How the tool sets in [../capabilities/agent.md](../capabilities/agent.md) are made,
in the words of [../architecture.md](../architecture.md). Each table lists the actions
still to build; for the rest the `Command` enum is the truth.

## Multi-agent

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

| Still to build | Does |
|---|---|
| `agent.reload` | end, then start with `--resume` |
| `agent.approve` · `agent.refuse` | `approvals.resolve`, then the row and the timeline |
| `agent.auto_approve` | flips the session's flag |
| `agent.get_activity` · `agent.get_asks` | reads for the MCP tools |

The review sheet is ui state, not a command.

**Rail** reads the slice and nothing else; the feed reads `timeline` for the opened
sessions.

**Removed.** The agents sidebar, the pop-out window and its bridge, dialogs and
notifications as surfaces, the frontend `autoApprove` short-circuit.

## MCP tools

**Module `tools`**: the definitions, the argument types, the harness descriptions.
Each tool names the controller function it calls; a test asserts the map is total.

**`mcp-server`**, ui layer: a tool call becomes a `Command` and goes through `dispatch`
like a click. The response is the controller's result. Writes pass
`approvals` inside the controller, so the blocking-until-resolved contract stays.

**Scoped to the session**: the SSE URL carries the session id, the token comes from the
launch, and every controller called through the server takes that session as its
default.

**Harness description**: kept as the single source. The core prompt and the skills
point to the tools; they never restate how to write.

## Activity

**Module `hooks`**: the receiver, mounted on `mcp-server`. Each POST becomes
`Event::Agent(Hook { session, kind, tool })` on the proxy. Bursts on file-editing
tools are coalesced before they reach the loop.

**Module `activity`**: the fold from hook events to status, used by the `agent`
service's `apply`.

**Module `timeline`**: one table — session, time, kind, subject, payload — indexed the
way it is read, newest first, with the row's own id breaking a tie inside one second.

**What belongs in it**: what a session did to its own work. Git — commit, push, pull,
rebase. The forge — an MR opened, written again, merged or closed, a CI result, a review.
Its notes, its repos and its worktrees. And the turns around them, so the feed can say
when the agent was working. **What does not**: the tools the agent ran on files. A write
or a shell command is the agent's status, which the rail shows live, not its history.

Written by `apply` for the turn kinds, by the controllers that make each action, and by
`approvals` for an approved write, which is one of those actions by another name. Read by
the rail's feed and the session's timeline. A stored kind this version does not know is
skipped rather than guessed at.

## Core prompt

**Module `agent-launch`**: the template and the render. The approval rule reads "a write
may wait for a human". Passed per launch, never persisted into the conversation.

## Skills

**Module `skills`**: the two directories, the list with hints, read, save, delete,
the stale flag. Controllers `agent.list_skills`, `agent.read_skill`, `agent.save_skill`,
`agent.delete_skill`, `agent.send_skill`. `agent.save_skill` from the agent asks like
any write.

**Buttons.** A worktree row's delivery state names its skill; the button sends
`agent.send_skill` with it. The agent pane keeps one menu for the rest.

## Needs

- [x] `SessionActivity` in `types`: status, asks, auto-approve, last change, last seen.
- [x] `seen_at` and `auto_approve` on the `session_state` row.
- [x] Timeline table and its migration.
