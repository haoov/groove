# Tasks — implementation

How the tool sets in [../capabilities/tasks.md](../capabilities/tasks.md) are made,
in the words of [../architecture.md](../architecture.md). Each table lists the actions
still to build; for the rest the `Command` enum is the truth.

## Multi providers

**Groove reads a task; it writes only what it did.** Opening a session sets the status
in progress, finishing sets it done, and `task.log_hours` adds the hours Groove
measured. A new task, a changed body, any other property: the agent asks for it through
`approvals`, and the review sheet shows it. The board and the overview never edit.

**The six properties**, named per source in the config, with what each provider calls
them: status, priority, start, due, estimate, and the one hours are logged to. A name
left out is a gap, not an error — Setup lists the six and what a gap costs. `status_map`
and `priority_map` say which of the source's own values mean what, and the first value
of each is the one Groove writes.

**Module `provider`**, on `http`: one `Source` per provider, read-only — list, fetch,
body. The enum is the registry: a new provider is a new arm, and the compiler asks for
it everywhere at once. **Service `task`** holds the list in its slice.

**GitHub** reads the open issues assigned to you that sit on a project board, and takes
the six from the board's fields. Its token comes from `gh auth token`, or from the
config when a host has one of its own.

| Still to build | Does |
|---|---|
| `task.get` · `task.list` | reads for the MCP tools |
| `task.body` | the task's body, as the overview shows it |

## Tasks management

**Service `task`**: the list with each task's provider fields, the local order, the
due and start dates and duration read as properties, the attention state per item.
The board reads this slice and the `session` slice; nothing else.

**Board**, in `ui`, as [../design.md](../design.md): a surface of its own beside the
rail, reached from the rail's Board row or `ctrl+shift+K`, left by the same chord or by
picking a session. It is a surface, not an overlay: while it is up the rail holds no
selection, and with no session open it is the window — closing the last one lands
there, and its Live column says how to start. Three columns, each scrolling on its own:
Live reads the open sessions, Up next the tasks no session holds, Review waits for the
forge. Still to draw: the filter, folded and expanded items, the plan's order and its
*later* divider, the attention lines, the timeline band.

**Attention** is computed in the service from `forge` facts and dates against the
thresholds in Config › Preferences: a review waiting, changes requested, CI failed,
approved and unmerged, due soon, overdue. Each yields a reason and an age; the rail's
Board row sums them.

| Still to build | Does |
|---|---|
| `task.open` | create or load the session, then `session.open` |
| `task.finish` | status done, teardown, remove from the rail |
| `task.delete` · `task.delete_local` | at the provider; locally only |
| `task.set_status` | by lifecycle only: in progress on open, done on finish |
| `task.log_hours` | the unlogged hours to the ledger and the source |
| `task.get_time` | read: tracked and logged hours |
| `task.reorder` | move an item in Up next; above or below *later* |
| `task.create` · `task.set_property` · `task.set_body` | the agent's own writes, through `approvals` |

The board's **+ task** opens an explorer; filing the task is what the agent does from
it.

The timer is the `task` service's own: it credits the focused session on a clock
while the window has focus and there is input or a busy agent. No controller.

**Removed.** Pause. The activity heatmap and `get_activity_days`. *Blocked by*.

## Needs

- [x] The six properties named per source, with a status map and a priority map.
- [ ] Local order per task and the *later* position, in the database.
- [x] Attention rules and thresholds in `types`.
- [ ] The attention fold in the `task` service.
- [ ] Notion on the same `Source` enum: its row, its mapping, its body read.
