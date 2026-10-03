# Sessions

Keep a task's repos, worktrees and agent together. Surface: the session's overview and
agent pane, as [../design.md](../design.md#session--small-scope) draws them.

Four kinds — [task](../glossary.md#task), [explorer](../glossary.md#explorer),
[review](../glossary.md#review) and [routine](../glossary.md#routine-session) — on one
provisioning path. A routine session holds no repo and no worktree: its agent pane alone,
full width, under the rail's Routines heading and never on the board.

## Repos management

**A clone is never written to.** Worktrees are cut from it, and `git fetch` refreshes it
before each cut. A repo the pool lacks is cloned into it.

## Worktrees management

**One worktree a repo a session**, on a branch Groove names from the task, cut from a
source branch and measured against a target branch. Both pickers list `origin`'s heads.

**Closing is not deleting.** A closed session keeps its worktrees, its repos and its row.
It leaves the rail and stays in the board's Live column; picking it there puts it back
on the rail with its agent. Only `session.delete` takes it away.

**A review session tracks the MR's own branch** rather than cutting one. Its id comes
from the project and the MR number, so opening the same MR again finds the session it
already has.

## Overview

One overview for task, explorer and review sessions: the task's properties, the repos
with their worktree rows, then the body. An explorer has neither properties nor body.

A worktree row joins its git status to its MR. Its skill button sends the skill that
fits the row's state, with `agent.send_skill`. Finish is offered when every worktree is
merged or closed.

## Agent session

**The pane draws the agent's terminal**; keys go to the pty as bytes. Escape reaches the
agent's TUI like any key. The action bar holds reload, the skills menu and the
auto-approve switch. An ask stands on the rail row alone.

**Skills are scoped per launch**: the pane's menu lists exactly what this agent was
started with. A skill saved later shows after a reload.
