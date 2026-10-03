# Agent

Run one agent per session, see what each does, and approve what it writes. Surface: the
rail, as [../design.md](../design.md#rail--mid-scope) draws it.

## Multi-agent

**One status per session's agent**, read from its hooks and its exit:

| Status | From | Until |
|---|---|---|
| working | `PreToolUse`, `PostToolUse`, `UserPromptSubmit` | `Stop` |
| asking | `Notification`; `PreToolUse` of `AskUserQuestion` | the next prompt or tool use |
| done, unseen | `Stop` | the session is selected on the rail |
| idle | at start; done and seen | the next prompt |
| exited | the pty exits non-zero | reopen |
| error | a failed action on the session | the next one that succeeds |

The done row says what changed since the user last looked: commits and files, from git.

**Attention class**, from the status: needs you — asking, an ask, exited, error; act when
you look — done unseen; moving — working; quiet — idle. Colour and motion follow
[../design.md](../design.md#rules).

**Every write the agent sends waits as an [ask](../glossary.md#ask)**, shown on its rail
row: the op and its subject, Approve and Review. With
[auto-approve](../glossary.md#auto-approve) on, the write is approved as it arrives and
recorded. The switch is per session; a new session takes the default from
Settings › Agent.

## MCP tools

**A tool is a controller function.** A read answers from the state; a write goes through
the approvals queue, and the call blocks until the user decides.

**Scoped to the session.** The tool server's URL carries the session id, and every tool
takes that session as its default.

**The tool descriptions are the one source** of how the agent writes. The core prompt and
the skills point to the tools; they never restate them.

## Activity

**The timeline holds what a session did to its own work**: git — commit, push, pull; the
forge — an MR opened, written again, merged or closed, a CI result, a review; its notes,
repos and worktrees; and the agent's turns, so the feed can say when it worked.

**It does not hold the tools the agent ran on files.** A write or a shell command is the
agent's status, which the rail shows live, not its history.

A stored kind this version does not know is skipped rather than guessed at.

## Core prompt

**Passed per launch, never stored in the conversation.** It names the session and the
rules; its approval rule reads "a write may wait for a human".

## Skills

Core skills (`groove`), the user's own (`user`), and the team's (see
[../shared.md](../shared.md)). The agent reads and writes its own skills through tools;
a save asks like any write. A worktree row's state names the skill that fits it, and its
button sends that skill.

## Shared skills, knowledge, routines

As [../shared.md](../shared.md) describes them.
