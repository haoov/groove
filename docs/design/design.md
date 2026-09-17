# Design

Three surfaces, three scopes, and settings beside them. Information never leaks
upward: what belongs in the session does not reach the rail; what belongs on the
board does not reach the rail.

| Surface | Scope | Holds | Capability |
|---|---|---|---|
| Board | large | every task, live or not, with its repos and MRs | Tasks |
| Rail | mid | the sessions actually open, and what their agents do | Agent › Multi-agent |
| Session | small | one session, in full | Sessions, Workspace |
| Settings | whole window | setup, providers, appearance, preferences | Config |

## Rules

- **Colour means state. Motion means activity.** One colour per state and nothing
  else uses colour: an ask is peach, a working agent is blue, CI and diff carry green
  and red. Text carries the colour, never a background. If it moves, an agent works.
- **A fact clears when the world changes, never when the user looks at it.** No
  dismiss.
- **Suggest, never auto-send.** The action of a state is the skill that fixes it,
  shown as a button. Groove never runs it by itself.
- **Everything is reachable from the keyboard.** The command palette is the front
  door to every action in every capability. The mouse is a shortcut.
- **Selection is a background, never an underline or a bar.** A selected tab, row or
  item takes a filled background one step above its ground.

## Board — large scope

The board replaces the agent pane, the workspace and the sidebar; the rail stays. It
uses that width as three columns, one per list, each scrolling on its own.

**Header.** The filter — `field:value` tokens and bare words, with autocomplete —
applied to all three columns at once. On the right, **+ task**: a new task at the
provider, or a new explorer.

| Column | Holds | Sort |
|---|---|---|
| **Live** | sessions with a worktree | last activity |
| **Up next** | tickets with no session, in the user's local order — the plan | the user's order |
| **Review** | the forge queue: MRs where the user is a requested reviewer | updated |

Each column header carries its count.

**Live item.** Folded: kind icon, title, repo count, twisty. Expanded: one row per
worktree — branch, then git counts, MR, CI and notes as icons, zero and absent
omitted — and a link to the forge. The title opens the session, which joins the
rail. Right click: open in provider, finish, delete, delete locally.

**Up next item.** A position number, title, priority and size as text. Drag to
reorder. One divider, **later**, that items can be dragged under, so the top of the
column stays short. The order is Groove's and is never written to the provider. A
ticket that gets a session leaves the plan for Live.

**Review item.** Project and MR number, title, author, updated. Open creates a review
session, which joins the rail.

**Attention.** An item that needs the user gets one line under its title — the reason
and the age, in peach — and floats to the top of its column whatever the sort:

```
◎ paxone-deploy !88          waiting 5d
⚑ Harden Groove…             changes requested · 2d
⚑ finops export              CI failed · 3h
```

The rules, each with an age and a threshold in Config › Preferences: a review
waiting longer than the threshold; an MR with changes requested; an MR with CI
failed; an MR approved and green but not merged for a day. The rail's Board row
carries the count of items with attention, in peach, and nothing else.

**Timeline.** A band under the three columns, full width, collapsible; collapsed to
its header by itself when nothing falls in the horizon. Four weeks, today about a
quarter in, days as hairlines, week boundaries stronger, weekends dimmed. One bar per
task: from its start date to start plus duration, or to its due date when it has
one; a point when it has only a due date; an open bar to today when it has only a
start. Bars stack when they overlap. The filter applies to the band. Dates and
duration are properties, read from the provider and edited by hand in the overview.
The band never reorders Up next.

Overdue and due soon are attention rules like the others — *due in 2d*,
*overdue 3d* — a peach line on the item in its column and a peach bar on the band.

No activity heatmap. No *blocked by*.

## Rail — mid scope

Only the sessions actually open, in the order opened. Never re-sorted. The Board
row above them carries the board's attention count when it is not zero.

Each row:

1. **Type icon** — task, review, explorer.
2. **Title.**
3. **Agent status** — `working`, `committed 2 · 5 files`, `idle`, `exited`,
   `error`. One line, truncated, never wrapped.
4. **Agent action, with buttons when it needs one** — `asks to commit`
   **Approve** **Review**.
5. **Relative time, right-aligned** — how long the agent has waited, or since it
   finished.

```
 ▤  Board
 ─────────────────────────────────────────
 ⚑  TASK-51  auth refresh                2m
    asks to commit         [Approve] [Review]
 ⚑  fix/keys-50                          6m
    committed 3 · 7 files
 ⚑  TASK-49  cert rotation              12m
    committed 2 · 5 files
 ◎  review-3  paxone argo               now
    working
 ⚑  TASK-48  finops export               3h
    idle
```

Nothing else: no CI, no MR state, no worktree count.

An ask's text is peach and offers *Approve* and *Review*. With auto-approve on for
the session, nothing asks. A working status is blue and its glyph moves. No row has a
background of its own.

Opening a session starts its agent. Closing a row ends the agent and removes the row.
The session stays as it is on disk.

**Feed.** Below the rows, the event log of the opened sessions, newest first,
filterable to the selected session: agent turns, commits, pushes, MR events, CI
results, notes. Monochrome, no motion. Asks are never in the feed; they live on the
row. Collapsible.

## Session — small scope

One window, four columns, left to right. The left half is the agents; the right half
is the work.

| Column | Holds |
|---|---|
| **Rail** | opened sessions, the feed |
| **Agent pane** | the agent's PTY · action bar: skills menu, reload, the ask with Approve and Review |
| **Workspace** | header · tabs · the selected tab · the manual section |
| **Sidebar** | contextual list for the selected tab; folds away |

The rail, the agent pane and the sidebar each have a width the user drags. The
workspace takes what is left, so it is the only column a window resize, a fold or a
drag of a boundary it does not touch ever changes. A boundary moves the two columns it
stands between and nothing else. The sidebar folds from the far end of the tab strip
or with `ctrl+shift+B`, whatever the tab offers.

### Workspace

**Header.** One line across the agent pane, the workspace and the sidebar: type
icon, title, the repo and worktree pickers — the session's selector, which every tab
and the manual section follow — the selected worktree's MR and CI, a refresh button
that reloads the worktree at once, then the task actions: finish, and a menu with
delete and open in provider.

**Tabs.** `overview · diff · editor`.

| Tab | Shows | Sidebar |
|---|---|---|
| overview | properties; then the repos, each with its worktrees as rows — branch, git status, MR, CI and notes as icons and counts, zero counts and absent MR or CI omitted, no words, and the row's skill button; then the body. Properties and body edited by hand. Close task when every worktree is merged or closed | folded |
| diff | the change on one code surface, in three views, notes inline; default when the session has changes | a search bar — files, or grep with `/` — then three tabs: files — changed files as a tree, list by right click, with stage, unstage, discard, and the commit box under them; commits — the list, a commit opens its diff; notes — the session's annotations and threads, a note opens its line |
| editor | any file, on the same surface; a changed file keeps its marks and its views | file explorer, search, grep results |

**One code surface.** The diff and the editor are one surface in three views, which
the gutter holds together.

**Where the caret is** is two rules, above its row and below it, across the surface
— not a ground. The rows already carry grounds for what a change did, and what a
caret holds is a ground of its own; a third would leave the three telling each other
apart by shade.

**Editing is modeless, and stays usable by anyone.** Keys do what they do everywhere
else: characters type, arrows move, shift holds, `ctrl+c/x/v` carry, `ctrl+z` undoes,
`ctrl+s` writes. Tab writes what the language's own formatter writes — a tab in Go,
two spaces in YAML and Markdown, four elsewhere — so a file keeps the shape its tools
give it.

A **modal layer comes after settings**, and it is a grammar over what the editor
already holds rather than a second editor: the buffer keeps a set of selections, one
per caret, so several carets and a normal mode are additions to that set. It will not
be an emulation of vim — no ex commands, since the palette is the command line — and
the modeless keys above stay whatever else lands on top of them.

| View | Shows | Gutter |
|---|---|---|
| file | the file as it is now | a mark on every line the change touched |
| inline | the change as one column | the old and the new line number |
| split | the old beside the new | one number per side |

The new side is editable in every view. A caret lives in the document, never in a
row, so it stays where the user left it while the rows move under it. In the inline
view a removed line belongs to the old document and takes no caret.

**Finding your way in a big change.** A change over forty files in twenty directories
is where a diff is won or lost. Four answers, and none of them is a bigger tree:

- **The path, once.** The file list strips the whole change's common root and shows
  it once, then collapses every directory chain with a single child. A file reads as
  its name first and the rest of its path behind it, dimmed. Where the name carries no
  meaning, `mod.rs` and its like, the directory is the name.
- **Where am I.** Two lines pin at the top of the surface: the file, and under it the
  scope the visible rows sit in, read from the syntax tree.
- **Crossing a directory.** In a scroll over several files, a band marks the point
  where the path changes, showing only the segments that differ from the file above.
- **What is left.** One column down the edge holds the whole change: a band per file,
  its height its diff's length, its hunks as marks, the viewport as a lens that drags.
  A file marked read dims; a file carrying a note is marked. What is left is what is
  still bright. The marks belong to the session and outlive the window.

**Manual section.** Where the user acts by hand, as the agent pane is where the agent
acts. Terminals for the selected worktree, splittable, resizable. Collapsible; hidden
entirely when collapsed.

**Commit box.** In the sidebar under the changed files, since it acts on the staged
set the list shows: git status, message, commit, and an actions menu — push, pull,
rebase, discard all. Commit commits the index.

### Review sheet

An ask's **Review** opens a sheet over the work half — workspace and sidebar. The
agent pane stays visible. One frame for every op: the repo and the branch, then the
text of the write. No diff and no file list — the code was reviewed before this point.
A push goes to the worktree's own branch and nowhere else.

| Op | Text shown |
|---|---|
| commit | the message: title and body |
| push | each commit's title and body |
| MR create, update, close | the MR's title and body |
| task property | the task's title, the property, before → after |
| task body | the task's title and the body |
| task finish | the task's title and the worktrees torn down |
| discard | the files |

**Approve** in peach, **Refuse**. No editing in the sheet. Esc closes it without
deciding.

### Board and settings

The board replaces the agent pane, the workspace and the sidebar; the rail stays.
Settings takes the whole window.
## Settings

Config's surface. Takes the whole window, the rail included; Esc or *back* returns to
where the user was. Reached from the rail's footer and the palette. A section list on
the left — Setup, Providers, Appearance, Preferences — with a search bar at its top:
typing filters every section to the matching rows, each shown with its section. The
selected section's form on the right. Labels left, controls
right, one row per setting, hairlines between groups, paths and ids in mono.

| Section | Rows |
|---|---|
| Setup | environment check — git, gh, glab, claude — each with its version and a mark · claude login · the config file, the state database and the worktree root as paths |
| Providers | task source, Notion or GitHub, with its fields · forge tokens, gh and glab, present or missing |
| Appearance | theme — Latte, Mocha, system · UI font · agent font · font size |
| Preferences | suggest actions · auto-approve default · attention thresholds in days: review waiting, due soon, approved unmerged · git: clone pool path |

Every change saves to the config file on the spot. No save button.
