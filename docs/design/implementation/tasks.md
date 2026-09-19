# Tasks — implementation

How the tool sets in [../capabilities/tasks.md](../capabilities/tasks.md) are made,
in the words of [../architecture.md](../architecture.md). Each table lists the actions
still to build; for the rest the `Command` enum is the truth.

## Multi providers

**Module `provider`**, on `http`: the trait, the two implementations, the registry,
the generic writes. **Service `task`** holds the task list and each task's properties
and body in its slice.

| Still to build | Does |
|---|---|
| `task.sync` | refetch one task from its source |
| `task.get` · `task.list` | reads: one task with properties and body; the list |
| `task.schema` · `task.relation_options` · `task.template` | reads, for the overview's editors and the agent |

The compile-time registry rule stays; a provider is added by a row.

## Tasks management

**Service `task`**: the list with each task's provider fields, the local order, the
due and start dates and duration read as properties, the attention state per item.
The board reads this slice and the `session` slice; nothing else.

**Board**, in `ui`, as [../design.md](../design.md): three columns; the filter with
the grammar that exists, applied to all three; items folded or expanded; the plan in
Up next with the *later* divider; attention lines; the timeline band.

**Attention** is computed in the service from `forge` facts and dates against the
thresholds in Config › Preferences: a review waiting, changes requested, CI failed,
approved and unmerged, due soon, overdue. Each yields a reason and an age; the rail's
Board row sums them.

| Still to build | Does |
|---|---|
| `task.open` | create or load the session, then `session.open` |
| `task.create` | at the provider; from the board or the agent |
| `task.finish` | status done, teardown, remove from the rail |
| `task.delete` · `task.delete_local` | at the provider; locally only |
| `task.set_property` · `task.set_body` | by hand from the overview; from the agent through `approvals` |
| `task.set_status` | by lifecycle — in progress on open, done on finish — or by hand |
| `task.log_hours` | the unlogged hours to the ledger and the source |
| `task.get_time` | read: tracked and logged hours |
| `task.reorder` | move an item in Up next; above or below *later* |
| `task.filter` | the board's filter string |

The timer is the `task` service's own: it credits the focused session on a clock
while the window has focus and there is input or a busy agent. No controller.

**Removed.** Pause. The activity heatmap and `get_activity_days`. *Blocked by*.

## Needs

- [ ] Local order per task and the *later* position, in the database.
- [x] Attention rules and thresholds in `types`.
- [ ] The attention fold in the `task` service.
- [ ] Start, duration and due read from the provider schema by property name,
      configured per source.
