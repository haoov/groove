# Sessions

Keep a task's repos, worktrees and agent together. Surface: the session's overview and
agent pane, as [../design.md](../design.md#session--small-scope) draws them.

Three kinds — [task](../glossary.md#task), [explorer](../glossary.md#explorer) and
[review](../glossary.md#review) — on one provisioning path.

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

**A write lands only where the session holds.** An object in a namespace needs that
namespace or the whole cluster; an object outside the namespaces needs the whole cluster.
A name or a namespace that holds a path or a query is refused, and a write reaches the
object or its scale, no other part. Enforced by
`a_session_on_one_namespace_writes_there_and_nowhere_else` and
`a_name_that_steps_out_of_its_path_or_a_part_groove_does_not_write_is_refused`.

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
why when it is not plain, and the MR when that work opened it. The agent writes it with
`log_work` only from `groove:save-task` or when the user asks, and only what matters to that
reader. A child keeps its own log; when it closes,
its newest entry goes to its parent's log, named after the child: one entry per child. Groove
keeps the log, and each entry's button on the overview takes a wrong one off it.
Enforced by `a_closed_child_hands_its_newest_entry_to_its_parent`,
`a_log_entry_removed_leaves_the_log_and_the_store` and `an_entry_s_button_removes_it_from_the_log`.

**A maintenance task reads the Platform team's firing alerts** from incident.io's API, at the
interval Settings › Alerts gives. incident.io filters them on the `Team` attribute's Platform entry, whose
ids Groove holds. The key is the one `inc auth login` left: Groove asks `inc` for it and stores
none, and the agent never runs `inc`. Settings › Alerts shows that login as the last read found
it, and its *log in* runs `inc auth login` on Settings' terminal. The config names the attributes read as cluster,
namespace, priority and environment. Enforced by
`a_task_marked_as_maintenance_reads_the_firing_alerts`,
`incident_io_is_asked_for_the_platform_team_s_firing_alerts_alone`, `a_read_follows_every_page` and
`alerts_show_the_inc_login_and_its_poll_and_log_in_runs_inc_auth_login`.

**An alert has one child.** Investigate opens the child the alert already has, or spawns one:
the alert as its brief, `groove:investigate` started, the alert's cluster and namespace attached
when Settings knows the cluster. Enforced by
`investigate_spawns_one_child_per_alert_on_the_alert_s_cluster`,
`an_alert_on_a_cluster_groove_does_not_know_attaches_none` and
`a_task_remembers_one_child_per_alert`.

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
