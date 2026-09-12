# Tasks — implementation

How the tool sets in [../capabilities/tasks.md](../capabilities/tasks.md) are made:
today, from the code; planned, in the words of [../architecture.md](../architecture.md).

## Multi providers

### Today

Two providers behind `TaskProvider`: Notion — a database, pages with properties, the
body as blocks round-tripped to markdown — and GitHub — issues, with Projects v2
fields as properties. `provider/mod.rs`'s `REGISTRY` is the single enumeration point,
sized by `ProviderId::ALL.len()`; a third variant refuses to compile until its row
exists. Nothing branches on a provider outside `provider/`. A task's key is
provider-typed: a Notion page id, or host, owner, repo and number. `short_id` is
minted once and names branches and worktree directories; a task's provider comes from
its row's column, never from the id's shape.

The trait: list, fetch, schema, properties, set property, relation options, set
status, discard, body read and replace, add hours, template, create. The schema per
source: the title property, properties with kind, options, relation target and
editability, status groups mapped to ready, in progress and done, the hours property
when the source has one. `provider/write.rs` holds every provider-generic write and
may not name a provider. Notion body writes are lossy for some block types and say
which. Setup detects the Notion property names and writes them to the config; a task
source can be turned on or off after first run.

### Planned

**Module `provider`**, on `http`: the trait, the two implementations, the registry,
the generic writes — unchanged in shape, moved. **Service `task`** holds the task
list and each task's properties and body in its slice.

| Controller | Does |
|---|---|
| `task.sync` | refetch one task from its source |
| `task.schema` · `task.relation_options` | read, for the overview's editors and the agent |

The compile-time registry rule stays; a provider is added by a row.

## Tasks management

### Today

Home: one snapshot per entry — short id, title, status, priority, provider, external
URL, kind, repos with worktree, branch, provisioned, missing and MR — folded into one
entry per session. Three sections: Live, Up next, Reviews; the last is the forge
queue. The filter grammar: `field:value` tokens and bare words, a repeated key is OR,
distinct keys AND, `-` negates, quotes carry spaces; fields id, title, provider,
forge, kind, status, priority, repo, branch, owner, author, approved, draft, mr;
autocomplete on keys and values. `FilterConfig` excludes statuses and filters by
assignee.

Actions: open, set active, finish — status to done, then teardown — delete at the
source, pause, set repos. Properties edited from the header pickers and the strip,
the body from its editor, both through the bridge into `write.rs`. A local timer on
the focused task ticks while the window has focus and there is recent input or a
busy agent; `log_task_hours` writes the unlogged hours to the local ledger and the
provider's hours field. Activity days feed a heatmap.

### Planned

**Service `task`**: the list with each task's provider fields, the local order, the
due and start dates and duration read as properties, the attention state per item.
The board reads this slice and the `session` slice; nothing else.

**Board**, in `ui`, as [../design.md](../design.md): three columns; the filter with
the grammar that exists, applied to all three; items folded or expanded; the plan in
Up next with the *later* divider; attention lines; the timeline band.

**Attention** is computed in the service from `forge` facts and dates against the
thresholds in Config: a review waiting, changes requested, CI failed, approved and
unmerged, due soon, overdue. Each yields a reason and an age; the rail's Board row
sums them.

| Controller | Does |
|---|---|
| `task.open` | create or load the session, then `session.open` |
| `task.create` | at the provider; from the board or the agent |
| `task.finish` | status done, teardown, remove from the rail |
| `task.delete` · `task.delete_local` | as today |
| `task.set_property` · `task.set_body` | by hand from the overview; from the agent through `approvals` |
| `task.set_status` | by lifecycle — in progress on open, done on finish — or by hand |
| `task.log_hours` | the unlogged hours to the ledger and the source |
| `task.reorder` | move an item in Up next; above or below *later* |
| `task.filter` | the board's filter string |

**Removed.** Pause. The activity heatmap and `get_activity_days`. *Blocked by*.

## Needs

- [ ] Local order per task and the *later* position, in the database.
- [ ] Attention rules and thresholds in `types`; the fold in the `task` service.
- [ ] Start, duration and due read from the provider schema by property name, configured per source.
