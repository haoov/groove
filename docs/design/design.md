# Design

Three surfaces, three scopes, and settings beside them. Information never leaks upward:
what belongs in the session does not reach the rail; what belongs on the board does not
reach the rail.

| Surface | Scope | Holds | Capability |
|---|---|---|---|
| Board | large | every task, live or not, with its repos and MRs | Tasks |
| Rail | mid | the sessions actually open, and what their agents do | Agent › Multi-agent |
| Session | small | one session, in full | Sessions, Workspace |
| Settings | whole window | setup, providers, agent, appearance, preferences, keymap | Config |

## Rules

- **Colour means state. Motion means activity.** One colour per state: an ask is peach, a
  working agent is blue, CI and diff carry green and red. Text carries the colour, never a
  background. If it moves, an agent works.
- **A fact clears when the world changes, never when the user looks at it.** No dismiss.
- **Suggest, never auto-send.** The action of a state is the skill that fixes it, shown as
  a button. Groove never runs it by itself.
- **Everything is reachable from the keyboard.** The palette is the front door to every
  action. The mouse is a shortcut.
- **Selection is a background**, one step above its ground — never an underline or a bar.

## Board — large scope

The board replaces the agent pane, the workspace and the sidebar; the rail stays. Three
columns, one per list, each scrolling on its own.

**Header.** The filter — `field:value` tokens and bare words, with autocomplete — applied
to all three columns at once. On the right, **+ explorer**, which starts one; filing
the task at the provider is the agent's own write.

| Column | Holds | Sort |
|---|---|---|
| **Live** | every session on disk, open or closed | last opened |
| **Up next** | tickets with no session, in the user's local order — the plan | the user's order |
| **Review** | the forge queue: MRs where the user is a requested reviewer | updated |

Each column header carries its count.

**Live item.** Folded: kind icon, title, repo count, twisty. Expanded: one row per
worktree — branch, then git counts, the MR's number, its checks and its notes, zero and
absent omitted — and a link to the forge. The title opens the session, which joins the
rail. Right click: open in provider, finish, delete, delete locally.

**Up next item.** A position number, title, priority and size as text. Drag to reorder. One
divider, **later**, that items can be dragged under. The order is Groove's and is never
written to the provider. A ticket that gets a session leaves the plan for Live.

**Review item.** Project and MR number, title, author, updated. Open creates a review
session, which joins the rail.

**Attention.** An item that needs the user gets one line under its title — the reason and
the age, in peach — and floats to the top of its column whatever the sort:

```
◎ paxone-deploy !88          waiting 5d
⚑ Harden Groove…             changes requested · 2d
⚑ finops export              CI failed · 3h
```

The rules, each with an age and a threshold in Config › Preferences: a review waiting
longer than the threshold; changes requested; a run that failed and stayed failed for ten
minutes; approved and green but not merged for a day; due soon and overdue. The rail's
Board row carries the count of items with attention, in peach, and nothing else.

No timeline, no activity heatmap, no *blocked by*: the task tools draw those better.

## Rail — mid scope

Only the sessions actually open, in the order opened. Never re-sorted. The Board row above
them carries the board's attention count when it is not zero.

Each row:

1. **Type icon** — task, review, explorer, routine.
2. **Title.**
3. **Agent status** — `working`, `asks you`, `committed 2 · 5 files`, `idle`, `exited`,
   `error`. One line, truncated, never wrapped.
4. **Agent action, with buttons when it needs one** — `asks to commit` **Approve**
   **Review**.
5. **Relative time, right-aligned** — how long the agent has waited, or since it finished.

```
 ▤  Board
 ─────────────────────────────────────────
 ⚑  TASK-51  auth refresh                2m
    asks to commit         [Approve] [Review]
 ⚑  fix/keys-50                          6m
    committed 3 · 7 files
 ◎  review-3  paxone argo               now
    working
 ⚑  TASK-48  finops export               3h
    idle
```

Nothing else: no CI, no MR state, no worktree count. No row has a background of its own.

An ask's text is peach and offers *Approve* and *Review*. With auto-approve on for the
session, nothing asks. A working status is blue and its glyph moves.

Opening a session starts its agent. Closing a row ends the agent and removes the row; the
session stays as it is on disk.

**Routines.** A standalone routine's session stands under its own heading, `Routines · N`,
folded by default, above the other sessions. Its row offers **Run**, which runs the routine
now, while no run is on it. Selected, it shows its agent pane alone, full width. It never
appears on the board.

**Feed.** Below the rows, the event log of the opened sessions, newest first, filterable to
the selected session: agent turns, commits, pushes, MR events, CI results, notes.
Monochrome, no motion, collapsible. Asks are never in the feed; they live on the row.

## Session — small scope

Four columns, left to right. The left half is the agents; the right half is the work.

| Column | Holds |
|---|---|
| **Rail** | opened sessions, the feed |
| **Agent pane** | the agent's pty · action bar: reload, skills menu, auto-approve switch |
| **Workspace** | header · tabs · the selected tab · the manual section |
| **Sidebar** | contextual list for the selected tab; folds away |

The rail, the agent pane and the sidebar each have a width the user drags, kept between
runs. The workspace takes what is left, so it is the only column a resize, a fold or a drag
elsewhere changes. A boundary moves the two columns it stands between and nothing else. The
sidebar folds from the far end of the tab strip or with `alt+shift+b`.

### Workspace

**Header.** Two lines, above the workspace alone — not the agent pane, not the sidebar.
The first holds the type icon and the title, with the task actions at its right: finish,
and a menu with delete and open in provider. The second holds the repo and worktree
pickers as buttons — the session's selector, which every tab and the manual section
follow — with the selected worktree's MR and CI and a refresh button at its right. A
title or a label too long for its line is cut with an ellipsis; the header never wraps.

**Tabs.** `overview · diff · files`.

| Tab | Shows | Sidebar |
|---|---|---|
| overview | the task's six properties, read-only; then the repos, each with its worktrees as rows — the branch, then git's counts and the MR's number, verdict, checks and notes at the row's right end, zero counts and an absent MR omitted; then the selected worktree's merge request, one property a line; then the body as text | folded |
| diff | the whole change as one stream, inline or split, notes inline, editable on its new side | the search bar — path and text, both live — then a strip that picks the list: files — the files that changed, a click scrolls the stream to one, with stage, unstage and discard by right click, and the commit box under them; commits — the branch's own log, a commit opens its change read-only; notes — the session's annotations and threads, a note scrolls to its line |
| files | the open files, one tab each with a dot while it owes the disk; the active one on the code surface, marked where the change touched it | the search bar, then the whole worktree as a tree with the path operations by right click; a click opens a file in the preview tab, in italic, which the next click replaces; a double click or the first edit keeps it. A found line opens at the line. A tab's right click offers close, close others and close all, which leave an unsaved file open; a middle click closes it |

**One code surface.** The Files tab's editor and the Diff tab's two views are one surface,
which the gutter holds together.

| Mode | Shows | Gutter |
|---|---|---|
| files | the file as it is now | a mark on every line the change touched |
| inline | the change as one column | the old and the new line number |
| split | the old beside the new | one number per side |

The new side is editable in every mode. A caret lives in the document, never in a row, so
it stays where the user left it while the rows move under it. A removed line belongs to the
old document and takes no caret. **Where the caret is** is two rules, above its row and
below it, across the surface — never a ground.

**Word diff.** A run of removed rows and a run of added rows of the same length pair one
for one, and each row shades the words its pair does not have. A line too long, or a pair
too far apart, is left to the row's own ground.

**Editing is modeless.** Keys do what they do everywhere else: characters type, arrows
move, shift holds, `ctrl+c/x/v` carry, `ctrl+z` undoes, `ctrl+s` writes. Enter opens the
next line at the indent of the one it left. Tab writes what the language's own formatter
writes — a tab in Go, two spaces in YAML and Markdown, four elsewhere.

A **modal layer comes after settings**, as a grammar over the selections the buffer already
keeps. It will not emulate vim, and the modeless keys stay.

**Search.** Two live rows in the sidebar: a path and a text search of the worktree. In the
surface, `ctrl+f` opens a bar over the file's own header, with the count at its end; enter
hands the keyboard back and keeps the session, `ctrl+n` and `ctrl+shift+n` step, esc ends
it. `ctrl+p` opens a file by its path.

**Finding your way in a big change.** Four answers, and none of them is a bigger tree:

- **The path, once.** The file list strips the whole change's common root and shows it
  once, then collapses every directory chain with a single child. A file reads as its name
  first and the rest of its path behind it, dimmed. Where the name carries no meaning,
  `mod.rs` and its like, the directory is the name.
- **Where am I.** The file pins at the top of the surface, and under it the scopes the
  visible rows sit in, stacked as they nest, read from the syntax tree.
- **Crossing a directory.** In a scroll over several files, a band marks where the path
  changes, showing only the segments that differ from the file above.
- **What is left.** One column down the edge holds the whole change: a band per file, its
  height its diff's length, its hunks as marks, the viewport as a lens that drags. A file
  marked read dims; a file carrying a note is marked. What is left is what is still bright.
  A file is marked read by the user, never by scrolling past it. The marks belong to the
  session and outlive the window.

**Manual section.** Where the user acts by hand, as the agent pane is where the agent acts.
Terminals for the session, opened in its own directory, splittable, resizable, collapsible;
its bar alone when collapsed.

**Commit box.** In the sidebar under the changed files, since it acts on the staged set the
list shows: git status, message, commit, and an actions menu — push, pull, discard all.
Commit commits the index. The message is typed on the same kind of buffer a file is, so a
caret, a selection and an undo work there too; Enter is a line of the message and
`ctrl+Enter` commits. A rebase waits for conflict resolution, which is its own feature.

**Three grounds, brightest first.** The work is the brightest the theme has: the
workspace, its header, the board's own header, the commit box, the agent pane's frame.
The bands sit under it: the rail, the sidebar, a column's heading, a menu, a band across
the rows. A terminal is the deepest — the agent's pty and the terminal widget, and
nothing else.

**Three grounds a row can take, and they never share a value.** Under the pointer is the
quietest, a selected row is stronger, and what a click acts on is stronger again. Where the
caret or the open file is, two rules stand instead of a ground: a place is not a state.

**A choice about one thing opens on that thing.** A picker draws the same rows the palette
does, filtered and keyed the same way, anchored under what was clicked, as wide as its
rows, nothing behind it dimmed, and it asks for a query only once the list is longer than
it shows. The palette stays in the middle of the window and dims it.

**One button says what to do now.** The commit box offers a single action — commit while
something is staged, push while ahead, pull while behind, open an MR once the branch is
landed and has none — with a caret beside it for everything else the worktree can do,
which includes writing that MR's text again or closing it. The menu hangs from that caret and ends on the rule
that separates the box from the list.

**A row offers, it never surprises.** Pointing at a file shows one word at its end — stage,
or unstage — where its counts were. Everything else is behind the right button.

**A question takes the place of what asked it.** Discarding one file turns its row into the
question; discarding every change turns the commit box's first line into it. Two answers at
the end, no modal, and nothing destructive within a click of something ordinary.

### Review sheet

An ask's **Review** opens a sheet over the work half — workspace and sidebar. The agent
pane stays visible. One frame for every op: the repo and the branch, then the text of the
write. No diff and no file list. A push goes to the worktree's own branch and nowhere else.

| Op | Text shown |
|---|---|
| commit | the message: title and body |
| push | each commit's title and body |
| MR create, update, close | the MR's title and body |
| task property | the task's title, the property, before → after |
| task body | the task's title and the body |
| task finish | the task's title and the worktrees torn down |
| discard | the files |

**Approve** in peach, **Refuse**. No editing in the sheet. Esc closes it without deciding.

## Settings

Config's surface. Takes the whole window, the rail included; Esc or *back* returns to where
the user was. Reached from the rail's footer and the palette. A section list on the left —
Setup, Providers, Agent, Appearance, Preferences, Keymap — with a search bar at its top:
typing filters every section to the matching rows, each shown with its section. The
selected section's form on the right: labels left, controls right, one row per setting,
hairlines between groups, paths and ids in mono.

| Section | Rows |
|---|---|
| Setup | environment check — git, gh, glab, claude, curl — each with its version and a mark · claude login · the config file, the state database and the worktree root as paths |
| Providers | task source, Notion or GitHub, with its fields · forge tokens, gh and glab, present or missing |
| Agent | auto-approve default · the shared repo, by URL and branch · the skills — core, the user's own, shared — each with its toggle · the routines — pause all, how many run at once, each routine and its triggers |
| Appearance | theme — Latte, Frappé, Macchiato, Mocha · UI font · agent font · three sizes: interface, editor, terminal |
| Preferences | attention thresholds — review waiting, due soon and approved unmerged in days, a failed run in minutes · the forge poll's interval and its stale threshold |
| Keymap | every action with its chords; a chord is rebound by pressing it |

Every change saves to the config file on the spot. No save button.
