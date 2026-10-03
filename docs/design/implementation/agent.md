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
| working | `PreToolUse`, `PostToolUse`, `UserPromptSubmit` | `Stop` |
| asking | `Notification`; `PreToolUse` of `AskUserQuestion` | the next prompt or tool use |
| done, unseen | `Stop` | the session is selected in the rail |
| idle | at start; done and seen | next prompt |
| exited | `pty_exit` non-zero | reopen |
| error | a failed controller on the session | the next successful one |

The done row's text comes from `git` since the last seen mark: commits and files.

**Asks.** With auto-approve off, every write the agent sends through `tools` becomes
an ask on the row: the op and its subject, Approve and Review. With it on, `approvals`
approves on arrival and records the write; the timeline gets the entry. The flag is
per session, held in the slice; a new session takes the default from Settings › Agent.

**Attention class**, computed in the service: needs you — ask, asking, exited, error; act
when you look — done unseen; moving — working; quiet — idle. Colour and
motion follow [../design.md](../design.md).

The review sheet is ui state, not a command.

**Rail** reads the slice and nothing else; the feed reads `timeline` for the opened
sessions.

**Removed.** The agents sidebar, the pop-out window and its bridge, dialogs and
notifications as surfaces, the frontend `autoApprove` short-circuit.

## MCP tools

**Module `tools`**: the definitions, the argument types, the harness descriptions, and
whether a tool writes.

**Module `mcp`**: the loopback server. The binary hands each call to the loop as a
continuation, and `controllers::tools::answer` answers it from the state: a read by its
name, a write through `approvals`, so the blocking-until-resolved contract stays. A test
asserts that every listed tool is one Groove answers.

**Scoped to the session**: the SSE URL carries the session id, the token comes from the
launch, and every controller called through the server takes that session as its
default.

**Harness description**: kept as the single source. The core prompt and the skills
point to the tools; they never restate how to write.

## Activity

**Module `hooks`**: a loopback server of its own. Each POST becomes
`Event::Agent(Hook { session, kind, tool })` on the proxy; the `agent` service's `apply`
folds it into the session's status.

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

**Module `skills`**: the core, user and shared skills, the list with hints, read, save,
delete, the stale flag. Controllers `agent.list_skills`, `agent.switch_skill`,
`agent.delete_skill`, `agent.send_skill`. The agent reads and writes
its own skills through the `read_user_skill` and `save_user_skill` tools; a save asks like
any write.

**Buttons.** A worktree row's delivery state names its skill; the button sends
`agent.send_skill` with it. The agent pane keeps one menu for the rest.

## Shared skills, knowledge, routines

As [../shared.md](../shared.md) describes them. **Service `agent`** keeps the shared repo's
copy (`shared.rs`) and the routine runs (`runs.rs`); **module `routines`** reads the routine
files. The `routine` controller turns what changed into runs at each look, within the cap;
`agent.run_routine` is the Run button.

## Needs

- [x] `SessionActivity` in `types`: status, asks, auto-approve, last change, last seen.
- [x] `seen_at` and `auto_approve` on the `session_state` row.
- [x] Timeline table and its migration.
