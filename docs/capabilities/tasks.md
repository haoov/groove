# Tasks

Pull your tasks from their providers and work them as sessions. Surface: the board, as
[../design.md](../design.md#board--large-scope) draws it.

## Multi providers

GitHub issues and Notion databases, each a `Source` of the `provider` module. A new
provider is a new arm of that enum, and the compiler asks for it everywhere at once.

**Groove reads a task; it writes only what it did.** Opening a session sets the status
in progress, finishing sets it done, and `task.log_hours` adds the hours Groove measured.
A new task, a changed body, any other property: the agent asks for it as an
[ask](../glossary.md#ask), and the review sheet shows it. The board and the overview
never edit.

**Six properties, named per source** in the config: status, priority, start, due,
estimate, and the one hours are logged to. A name left out is a gap, not an error —
Setup lists the six and what a gap costs. `status_map` and `priority_map` say which of
the source's own values mean what; the first value of each is the one Groove writes. A
task whose source names no start date starts the day its first session did.

**GitHub** reads the open issues assigned to you that sit on a project board, and takes
the six from the board's fields. Its token comes from `gh auth token`, or from the config
when a host has one of its own.

**Notion** reads the rows of one database that the assignee property gives you, less the
statuses the config excludes, in the sprint that is running. The short id comes from
the page's `unique_id`, the body from its blocks as plain lines. A due date that is a
range takes the range's end.

**The sprint** is a relation the config names. Groove follows it to the sprint database,
asks which rows carry the current label, and filters on them. The answer stands five
minutes. A sprint property the database lacks yields no filter, never one Notion would
refuse.

## Tasks management

**Three lists**, one column each on the board: Live, every session on disk; Up next, the
tasks no session works; Review, the forge's queue.

**The filter** holds bare words, matched against the title, and `field:value` tokens —
status, priority, project, provider, kind, repo — matched with case and word breaks
ignored. Every term must answer, in all three columns at once. A token naming a field an
item has no value for takes that item out. A Live item answers for the task its session
works.

**The plan is Groove's own.** Up next stands in the order the user gave, and no provider
is told. Dragging a row asks `task.plan` to put the task above the row it lands on.
Below the **later** divider a task keeps its place but leaves the plan. The shown order:
the placed tasks, then the ones no order names, then everything under the divider.

**Attention** is a reason and an age: a review waiting, changes requested, CI failed,
approved and unmerged, due soon, overdue. The forge's facts and the dates are folded
against the thresholds in Config › Preferences. Several worktrees of one task read as
one: the earliest wait, the worst run, and an approval only where all of them have it.
The fold runs again when the tasks are read and every time the clock is taken, so a date
rule turns with the day.

**Time is two counters, never one.** `tracked_seconds` is what Groove measured,
`logged_seconds` what the source has been told. The difference is what is left to log,
so logging twice cannot count the same hour twice.

**The timer credits the selected session's task** while the window has focus and either
the user acted in the last two minutes or its agent is working. No run longer than that
is trusted, so an absence credits nothing.

**`task.log_hours`** writes the difference to the source's hours field, and only then adds
it to `logged_seconds`.

**`task.finish`** tells the source the task is done, and only then takes the session
away: its agent, its worktrees, its row. Work that is not committed or pushed stops the
teardown, so nothing is lost by finishing early. It is offered while no worktree of the
session still carries an open MR. `session.delete_local` is the same teardown with
nothing said at the source; `session.force_delete` takes the unsaved work with it.

**The status is the lifecycle's alone.** Opening a task's first session sets it in
progress, finishing sets it done, and a status the source already holds is not written
again. Nothing in the UI sets a status by hand.
