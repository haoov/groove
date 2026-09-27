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
them: status, priority, start, due, estimate, and the one hours are logged to. A task
whose source names no start date starts the day its first session did. A name left out
is a gap, not an error — Setup lists the six and what a gap costs. `status_map`
and `priority_map` say which of the source's own values mean what, and the first value
of each is the one Groove writes.

**Module `provider`**, on `http`: one `Source` per provider, read-only — list, and a
fetch that brings one task with its body. The enum is the registry: a new provider is a
new arm, and the compiler asks for it everywhere at once. **Service `task`** holds the
list and the bodies in its slice.

**GitHub** reads the open issues assigned to you that sit on a project board, and takes
the six from the board's fields. Its token comes from `gh auth token`, or from the
config when a host has one of its own.

**Notion** reads one database's rows: the ones the assignee property gives you, less
the statuses the config excludes, and in the sprint that is running. The six come from
the page's own properties, the short id from its `unique_id`, and the body from the
page's blocks as plain lines. Its token is the config's.

**The sprint** is a relation the config names. Groove reads the task database to find
what the relation points at, reads that database to find its own status property, and
asks it which rows carry the current label; those page ids become one `or` of
`relation contains` terms. The ids stand for five minutes. A sprint property the
database lacks yields no term at all, never a term Notion would refuse the query for.

## Tasks management

**Service `task`**: the list with each task's provider fields, the local order, the
due and start dates and duration read as properties, the attention state per item.
The board reads this slice and the `session` slice; nothing else.

**Module `plan`**, on `db`: the order the user gave, one row a task, written whole in
one transaction. It is Groove's own and no provider is told. The service turns it into
the shown order: the placed tasks first, then the ones no order names, then everything
under the divider.

**Board**, in `ui`, as [../design.md](../design.md): a surface of its own beside the
rail, reached from the rail's Board row or `ctrl+shift+K`, left by the same chord or by
picking a session. It is a surface, not an overlay: while it is up the rail holds no
selection, and with no session open it is the window — closing the last one lands
there, and its Live column says how to start. Three columns, each scrolling on its own:
Live reads every session on disk — closed ones included, dimmed until picked — Up next
the tasks no session works, Review waits for the forge. A Live item opens on its twisty to show its worktrees with what git says about
each; an Up next item carries its place in the plan and opens its session when picked.

**The filter** is the header, with **+ explorer** beside it, which starts one. It
holds bare words, matched against the title, and `field:value` tokens — status,
priority, board, provider, kind, repo — matched with case and word breaks ignored.
Every term must answer, in all three columns at once; a token naming a field an item
has no value for takes that item out. A Live item answers for the task its session
works, so the task's own properties narrow it as they narrow Up next. `/` opens the filter, Escape clears it then leaves it, and the
rows it offers come from what the board itself holds.

**Up next** stands in that order. A row's place is its handle: a press on the number
takes hold, the rule follows the pointer, and the drop asks `task.plan` to put the task
above the row it landed on. Below the **later** divider a task keeps its place but
leaves the plan.

**The timeline** is the band under the columns: four weeks, today a quarter in, days as
hairlines with the weeks stronger and the weekends dimmed, and today's own line in
peach. One bar a task, packed into the first row it does not overlap: a bar between its
dates, a bar to today when it has only a start, a dot when it has only a due date. An
item that needs the user wears its edge in peach. The filter narrows the band, and the
band orders nothing. Its own bar folds it away, its top edge drags its height, which is
kept between runs, and a sideways turn of the wheel over it carries the horizon through
time, a day a turn. It folds itself when no date falls in today's horizon and the user has not moved
it. A bar names its task under the pointer.

**Attention** is folded in the service from `forge` facts and dates against the
thresholds in Config › Preferences: a review waiting, changes requested, CI failed,
approved and unmerged, due soon, overdue. Each yields a reason and an age. An item that
carries one stands one line taller, says why under its title in peach, and floats over
the ones that carry none; the rail's Board row sums them. The fold is read again when
the tasks are, and every time the ledger takes the clock, so a date rule turns with the
day. The forge facts are empty until `forge` lands, so only the dates speak today.

| Still to build | Does |
|---|---|
| `task.delete` | the task at the provider, and the session with it |
| `task.get_time` | read: tracked and logged hours |

The board's **+ explorer** starts one; filing the task is what the agent does from it.

**Module `ledger`**, on `db`: two counters a task, never one. `tracked_seconds` is what
Groove measured, `logged_seconds` what the source has been told, and the difference is
what is left to log — so logging twice cannot count the same hour twice. `today_*` is
the share of the day, which starts again when the day does.

**The timer** is the `task` service's own, and the app's loop turns it: it credits the
selected session's task while the window has focus and either the user acted in the
last two minutes or its agent is working. No run longer than that is trusted, so an
absence credits nothing. The ledger takes what it measured every minute.

**`task.log_hours`** writes the difference to the source's own hours field and only
then adds it to `logged_seconds`; the overview offers it as a button on the Logged
line, and the number GitHub answers with next is what the line shows.

**`task.finish`** tells the source the task is done, and only then takes the session
away: its agent, its worktrees, its row. Work that is not committed or pushed stops the
teardown and the status stands, so nothing is lost by finishing early. The header offers
it while no worktree of the session still carries an open MR, and a caret beside it
opens the rest. **`task.delete_local`** is the same teardown with nothing said at the
source, from that menu. `session.delete` is the explorer's way out, and that one
forces.

**`task.set_status`** is the lifecycle's alone: opening a task's first session sets it
in progress, and finishing sets it done. It writes the option the `status_map`'s
first name points at, and writes nothing when the source already says so. Nothing in
the UI sets a status by hand.

**Removed.** Pause. The activity heatmap and `get_activity_days`. *Blocked by*.

## Needs

- [x] The six properties named per source, with a status map and a priority map.
- [x] The status written back on open; on finish with `task.finish`.
- [x] Local order per task and the *later* position, in the database.
- [x] The ledger, the timer, and the hours written to the source.
- [x] Attention rules and thresholds in `types`.
- [x] The attention fold in the `task` service.
- [x] The forge's facts behind its four rules, per worktree and merged per task.
- [x] Notion on the same `Source` enum: its row, its mapping, its body read.
