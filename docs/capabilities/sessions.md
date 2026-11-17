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

**Taking a worktree away never loses work by itself.** Uncommitted changes refuse it, and so
do commits origin lacks, unless the worktree's MR merged: the forge holds them then, squashed
or not. *Force close worktree* takes it away whatever it holds.

**A review session tracks the MR's own branch** rather than cutting one. Its id comes
from the project and the MR number, so opening the same MR again finds the session it
already has.

## Clusters management

**A session holds cluster and namespace pairs, as it holds repos.** It attaches only a
context Groove knows, from Settings › Clusters, from the context panel or the palette, and
the user attaches the whole cluster. A pair names one namespace; no namespace is the whole
cluster, which replaces that context's namespaces; a namespace, which the agent can ask
for, replaces a cluster held whole. The namespace panel picks what the list shows and
holds nothing. The pairs stay with the session across restarts and go with it.

## Maintenance tasks

**The user marks a task as [maintenance](../glossary.md#maintenance-task) in Groove**, from the palette; nothing reads it off the
title. Only a task is marked. Enforced by `only_a_task_is_marked_as_maintenance`.

**Only a maintenance task spawns [children](../glossary.md#child-session).** A child is an explorer whose agent starts from
the parent's brief, and the brief is all it gets. The agent spawns with `spawn_session`, the
user from the palette. Enforced by `a_task_not_marked_as_maintenance_spawns_no_child` and
`a_maintenance_task_spawns_a_child_beside_it_and_remembers_it`.

**A child keeps its parent**, adopted as a task or not; a parent deleted leaves it without
one. Enforced by `a_child_adopted_as_a_task_keeps_its_parent` and
`a_deleted_parent_leaves_its_children_without_one`.

**A session keeps a log of what it did**, which the user reads when they come back: what,
why when it is not plain, the MR. The agent writes it with `log_work` when it lands the work
and when it closes; a child's entry goes to its parent's log, named after the child. Groove
keeps the log. Enforced by `a_child_logs_to_its_parent_and_the_parent_to_its_own_log`.

## Overview

One overview for task, explorer and review sessions: the task's properties, the repos
with their worktree rows, the clusters it holds, then the body. An explorer has neither properties nor body.

A worktree row joins its git status to its MR. Its skill button sends the skill that
fits the row's state, with `agent.send_skill`. Finish is a palette row.

A maintenance task adds its children, each with its agent's state or `closed`. A task or an
explorer then shows its log, the newest entry first, `Nothing logged yet.` while it is empty.
Enforced by `a_maintenance_task_lists_its_children_and_the_log_shows_who_wrote_each_entry` and
`a_task_not_marked_shows_no_children_and_an_empty_log`.

The clusters name what is attached and nothing else: each context once, in its hue, its
namespaces under it. No health, no counts. A session with none has no clusters section.

## Agent session

**The pane draws the agent's terminal**; keys go to the pty as bytes. Escape reaches the
agent's TUI like any key. The terminal takes the whole pane: no bar stands under it, and a
skill is sent from the palette. Auto-approve is a setting. An ask stands on the rail row alone.

**Skills are scoped per launch**: the palette lists exactly what this agent was
started with. A skill saved later shows after a reload, from the palette.
