# Design

Three surfaces, three scopes, and settings beside them. Information never leaks upward:
what belongs in a session does not reach the board, and what belongs on the board reaches
the rail only as its attention count.

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
- **Selection is a background**, one step above its ground. A selected tab adds its accent
  underline.
- **Focus is a line.** The region that holds the keyboard carries a 2px accent line along
  its bottom edge. Enforced by `the_focused_region_alone_carries_the_line`.
- **A region is one chord away.** `alt+1` to `alt+4` hand the keys to the rail, the agent,
  the workspace and the sidebar; the terminals keep their own chord. A second press folds
  the sidebar away. Enforced by `a_second_press_folds_the_sidebar_away`.
- **A list walks from the keyboard.** In the sidebar the arrows step through the rows, Enter
  does what a click does, Space stages a changed file, Left and Right fold a folder, and
  `[` and `]` change the list. `alt+c` puts the caret in the commit box.
  Enforced by `the_arrows_walk_the_changed_files_and_space_stages_one`.
- **The rail and the terminals answer too.** On the rail, Enter goes to the workspace and
  `a` and `v` approve or show the agent's ask. `alt+]` and `alt+[` step through the
  workspace's tabs, or the terminals' tabs while they have the keys. Enforced by `on_the_rail_enter_goes_to_the_workspace_and_a_and_v_answer_the_ask`.
- **Kubernetes walks from the keyboard too.** The kinds answer the arrows, a letter starts
  their search. In the list the arrows walk the rows; Enter opens one on Describe, `y` on
  its YAML, `l` on its logs. `ctrl+tab` and `ctrl+shift+tab` walk the open files or objects,
  `ctrl+w` closes the one shown.
  Enforced by `the_list_s_rows_walk_and_open_on_describe_yaml_or_logs`.
- **The board walks too.** `alt+1` is the rail, `alt+2` to `alt+4` the Live, Up next and
  Review columns. The arrows walk the column; Enter, or a click, opens the session and goes
  to it, `ctrl+Enter` or `ctrl+click` opens it and stays on the board.
  Enforced by `the_board_s_columns_walk_by_key_and_enter_goes_or_stays`.

- **Long lines wrap on a toggle.** With *wrap long lines* on (Settings › Appearance or the
  palette), the editor wraps each line to its width; the arrows step row by row, a click
  lands along the line, and nothing scrolls sideways. A note wraps in its card and while it
  is typed. Without wrap, the diff and the editor scroll sideways to keep the caret in view.
  Enforced by `down_steps_along_a_wrapped_line_before_the_next_one`.

## Limits

- **No accessibility layer.** Groove draws its own pixels and gives screen readers
  nothing. AccessKit (`accesskit_winit`) is the way in when it is needed. Every view
  draws through the kit, so the kit is the one place that would emit the nodes.
- **No input method.** Dead keys and Compose work; IBus and fcitx5 do not reach Groove.
- **No home-grown layout engine.** Boxes lay out with Taffy, through the kit. Lists and
  cell grids (terminals, code rows, list rows) keep fixed math inside the box Taffy
  gives them. Content shaped like a document opens in the browser or stays plain text.
  Groove never lays out flowing pages.

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
| **Review** | the forge queue: MRs where the user is a requested reviewer, neither approved by them nor open in a session | updated |

Each column header carries its count.

**Live item.** One line: the kind icon, blue while the session is open, the title, then
the repo count and the worktree count when it is not one. A click opens the session,
which joins the rail. Right click: delete locally.

**Up next item.** A position number, title, priority and size as text. Drag to reorder. One
divider, **later**, that items can be dragged under. The order is Groove's and is never
written to the provider. A ticket that gets a session leaves the plan for Live.

**Review item.** Title and updated, then under them the project and MR number, the
author and where the reviewers stand. Open creates a review session, which joins the
rail. The column heading sorts by title or by updated; a second click turns the order
round. Up and down select an item, Enter opens it, Esc lets it go.

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

Nothing else: no CI, no MR state, no worktree count. A row takes a tinted ground only while
its agent asks, in peach, or is done and not yet seen, in green.

An ask's text is peach and offers *Approve* and *Review*. With auto-approve on for the
session, nothing asks. A working status is blue and its glyph moves.

Opening a session starts its agent. Closing a row ends the agent and removes the row; the
session stays as it is on disk.

**Routines.** A standalone routine's session stands under its own heading, `Routines · N`,
folded by default, below the other sessions. Its row offers **Run**, which runs the routine
now, while no run is on it. Selected, it shows its agent pane alone, full width. It never
appears on the board.

**Feed.** Below the rows, the event log of the opened sessions, newest first, filterable to
the selected session: agent turns, commits, pushes, MR events, CI results, notes.
Monochrome, no motion, collapsible. Asks are never in the feed; they live on the row.
Back-to-back events of one kind from one session fold into one row with a count; a click
opens the run to one line per event.
Enforced by `back_to_back_events_of_one_kind_fold_into_one_row`.

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

**Header.** Three lines, above the workspace alone — not the agent pane, not the sidebar.
The first holds the type icon and the title, with the task actions at its right: finish,
and a menu with delete and open in provider. The second holds the repo and worktree
pickers as buttons — the session's selector, which every tab and the manual section
follow — with the selected worktree's MR and CI and a refresh button at its right. A git
mark leads the pickers' line. A third line, led by a cluster mark, holds the scope on
every tab: the contexts picked, a lone one in its hue, then the namespaces; before any
cluster, a context picker alone. Each picker opens a panel with a search: what the session
holds first, a check to show it and a × to detach it, then what it could attach. A pick
there attaches; a known context opens its namespaces; Enter shows one line alone, and
attaches it first when the session does not hold it. A title or a label too long for its
line is cut with an ellipsis; the header never wraps; a tab's status badges keep their room
before the cut, and a badge with no room left is dropped.

**Tabs.** `Overview · Diff · Files · Kubernetes`, the last while the session holds a cluster.

| Tab | Shows | Sidebar |
|---|---|---|
| overview | the task's six properties, read-only; then the repos, each with its worktrees as rows — the branch, then git's counts and the MR's number, verdict, checks and notes at the row's right end, zero counts and an absent MR omitted; then the selected worktree's merge request, one property a line; then the clusters it holds, each context in its hue with its namespaces under it; then the body as text | folded |
| diff | the whole change as one stream, inline or split, notes inline, editable on its new side | the search bar — path and text, both live — then a strip that picks the list: files — the files that changed, a click scrolls the stream to one, with stage, unstage and discard by right click, and the commit box under them; commits — the branch's own log, a commit opens its change read-only; notes — the session's annotations and threads, open, resolved or all, under the file each stands on and the MR's own last, two lines a note; a note scrolls to its line, a resolved one opened |
| files | the open files, one tab each with a dot while it owes the disk; the active one on the code surface, marked where the change touched it | the search bar, then a strip that picks the list: notes — the diff's own list, a note opening its file at its line; files — the whole worktree as a tree with the path operations by right click; a click opens a file in the preview tab, in italic, which the next click replaces; a double click or the first edit keeps it. A found line opens at the line. A tab's right click offers close, close others and close all, which leave an unsaved file open; a middle click closes it |
| kubernetes | the list tab, titled by its kind and its count, and a tab for each object opened from it; [Kubernetes tab](#kubernetes-tab) draws both | a search that narrows the kinds, then the kinds the contexts serve, under headings that fold |

**One code surface.** The Files tab's editor and the Diff tab's two views are one surface,
which the gutter holds together.

| Mode | Shows | Gutter |
|---|---|---|
| files | the file as it is now | a mark on every line the change touched |
| inline | the change as one column | the old and the new line number |
| split | the old beside the new | one number per side |

An added or a removed row tints its gutter and its numbers in its own colour, a step
stronger than its ground.
In split, a side with no line is hatched, gutter included.

Over the diff, a band holds the whole change — how many files, what it added and removed
— and the inline and split switch. Over a file, the band names the file.

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

**Search.** Two live rows in the sidebar: a path and a text search of the worktree. In the
surface, `ctrl+f` opens a bar over the file's own header, with the count at its end; enter
hands the keyboard back and keeps the session, `ctrl+n` and `ctrl+p` step, esc ends
it. `ctrl+shift+p` opens a file by its path.

**Finding your way in a big change.** Three answers, and none of them is a bigger tree:

- **The path, once.** The file list strips the whole change's common root and shows it
  once, then collapses every directory chain with a single child. A file reads as its name
  first and the rest of its path behind it, dimmed. Where the name carries no meaning,
  `mod.rs` and its like, the directory is the name.
- **Where am I.** In the diff, each file opens on a head two rows tall: its name, its
  folder dimmed after it, then its counts and its read box at its end. The head of the
  file the rows stand in pins at the top, over a soft shadow, until the next file's head
  pushes it out. In the file view, the scopes
  the visible rows sit in pin instead, stacked as they nest, read from the syntax tree.
- **What is left.** One column down the edge holds the whole change: a band per file, its
  height its diff's length, its hunks as marks, the viewport as a lens that drags. A file
  marked read dims. What is left is what is still bright.
  A file is marked read by the user, never by scrolling past it. The marks belong to the
  session and outlive the window.

**Manual section.** Where the user acts by hand, as the agent pane is where the agent acts.
Terminals for the session, opened in its own directory, splittable, resizable, collapsible;
its bar alone when collapsed.

**Commit box.** In the sidebar under the changed files, since it acts on the staged set the
list shows: git status, message, commit, and an actions menu — push, pull, discard all.
Commit commits the index. The message is typed on the same kind of buffer a file is, so a
caret, a selection, an undo and the clipboard work there too; Enter is a line of the message and
`ctrl+Enter` commits.

**Three grounds, brightest first.** The work is the brightest the theme has: the
workspace, its header, the board's own header, the commit box, the agent pane's frame.
The bands sit under it: the rail, the sidebar, a column's heading, a menu, a file's head.
The deepest holds the agent's pty, the terminal widget, a note's own rows under its card,
and the field a note is typed in — nothing else.

**Three grounds a row can take, and they never share a value.** Under the pointer is the
quietest, a selected row is stronger, and what a click acts on is stronger again. Where the
caret is, two rules stand instead of a ground: a place is not a state. The row a list stands
on — the open file, the shown commit, the session, the kind listed — has the two rules over
the held ground, the accent dimmed into it.

**A choice about one thing opens on that thing.** A picker draws the same rows the palette
does, filtered and keyed the same way, anchored under what was clicked, as wide as its
rows, nothing behind it dimmed, and it asks for a query only once the list is longer than
it shows. The palette stays in the middle of the window and dims it.

**The palette keeps its groups while it filters.** Each group runs under its heading, the
group with the best match first. A first word that starts a group's name keeps that group
whole and on top: `go` lists every Go entry, `git pu` finds Push.
Enforced by `a_group_named_first_comes_first_and_whole`.

**A step names what it asks.** An entry that asks several questions shows the current one,
dimmed, before the caret: `repo`, then `branch`, then `target`. Backspace on an empty field
takes the last answer back into it, and on the first question returns to the entries.
Enforced by `backspace_on_an_empty_query_takes_the_last_answer_back`.

**`#` searches files.** A query that starts with `#` lists the selected worktree's files, a
match in the file name first, and opens the one picked on the Files tab. A line under the
input names the session, repo and branch the palette acts on.
Enforced by `a_hash_searches_the_worktree_files_and_opens_the_one_picked`.

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

### Kubernetes tab

- **List.** The list tab, titled by its kind and its count, over the rows of every context
  and namespace in scope: the server's Table columns `kubectl get` shows, behind a cluster
  column and a namespace column where those tell rows apart; a status and a ready count in
  the colour of how the object stands. A time cell, the age or a last schedule, counts up
  live; the age from the object's creation. A header sorts by its column, numbers and ages
  by value, and a second click turns the order round; its edge drags the column's width,
  kept for the kind. `ctrl+f` opens a find bar over the rows: fuzzy name words —
  `pyth-ca-st` keeps `pythie-cayzn-staging` — and `label=value` words the server filters
  on. `ctrl+shift+n` opens the namespace panel.
- **Object.** A row opens its object in a tab of its own beside the list, `describe ·
  yaml`, and `logs` for a pod. The tab reads the object, its events and its owners by name,
  and a pod's usage every 15 s, while it shows. As YAML it is the file editor's own view of
  the object, read-only.
- **Describe.** Described, it is one column of sections, each folding under its heading:
  conditions, a summary of facts a line each, a pod's containers, labels, annotations,
  relations and events. Its relations are the list's table under headings that fold (what
  owns it with its ready count, what it uses and how — mounted at a path, read as env by a
  container, pulling images —, what uses it); its containers the list's table (state,
  ready, restarts, the last end, CPU and memory each a bar of the usage against the limit
  with the request a tick, both told under the pointer) whose rows unfold into the image,
  probes, ports and mounts a line each, the ones not ready unfolded. Annotations, relations
  and events start folded. A name, an address, an image, a label or an event's message
  copies on a click: the copy mark shows on hover and turns to a green tick once copied. A
  link copies the same way; `ctrl`+click opens its object.
- **Logs.** Its logs are the file editor's view too, read-only, at the small text size: a
  tab per container and one for all of them, the current run or the previous one, the last
  5 minutes to 24 hours or from the start, each line after its clock; a line naming an
  error is red, a warning yellow. The view follows the last line until the wheel moves it
  up; the footer then counts the lines below and follows them again on a click. A stream
  holds 10 MB, its oldest lines going first, and picks up after a break from its last
  second.
- **Argo CD.** Argo CD's Applications list their project, sync, health, auto-sync and
  revision. An Application's describe holds its last sync, red when it failed and blue
  while it runs, its heading included; its conditions; where it syncs to and how; and its
  sources, a line each with the revision it synced to. The summary names the context the
  destination stands on: the Application's own for the in-cluster address, else the one
  added whose server matches, read once from the kubeconfig without a sign-in check. A
  resource's row opens its tab there. A git source's address, ctrl+clicked, opens the
  files on its repo, selected in the session or added from the pool, cut from the branch
  the source follows when it is one; its synced commit, ctrl+clicked, shows in the diff as
  a commit of the list does, fetched first when the clone lacks it. Its `resources` view
  filters by what needs attention (out of sync, to prune, or failed in the last sync), out
  of sync, syncing or all, each in its colour with its count; it opens on what needs
  attention, or on the sync while one runs. Both it and syncing are the list's table, its
  headers sorting each group's rows and its edges dragging widths kept for both. Resources
  stand by kind, each kind a heading on the band's ground, the kinds needing attention
  first; under all, a kind with nothing to see stands folded. Syncing shows the sync's line
  over a bar that fills resource by resource, then the PreSync hooks, each wave in order
  with its Sync hooks on a badge, and the hooks after, each step applied, applying, waiting
  or failed; a stage done stands folded.
- **Sidebar.** A search that narrows the kinds — `ctrl+shift+p`; a word in a kind, a
  heading or an API group, `rbac/role` a heading and a kind — then the kinds the contexts
  serve, indented under headings that fold: workloads, network, config, storage, RBAC and
  other for Kubernetes' own, one a CRD group by its name, then cluster for the built-in
  kinds not namespaced. In the search, ↑ and ↓ step through the headings and the kinds;
  Enter folds a heading or lists a kind. No counts, no health marks.

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
Setup, Providers, Clusters, Agent, Appearance, Preferences, Keymap — with a search bar at its top:
typing filters every section to the matching rows, each shown with its section. The
selected section's form on the right: labels left, controls right, one row per setting,
hairlines between groups, paths and ids in mono. Clusters alone is a table, since each
context holds several settings.

**Setup opens on its own once.** At start Groove checks the environment; when a required
program is missing, claude is signed out or no config file stands, Settings opens on Setup,
once a launch, unless the user acted first. A section that still needs something shows its
count in the list. Enforced by `setup_opens_once_when_the_first_check_finds_something_left`.

| Section | Rows |
|---|---|
| Setup | environment check — git, gh, glab, curl — each with its version and a mark · claude login · the config file, the state database and the worktree root as paths · next, Providers, while no task source stands |
| Providers | each task source, Notion or GitHub: its toggle with its gaps counted, where it is read from, then its fields, its status values and its priority values under their own headings, each a picker with what its gap costs beside it · forge tokens, gh and glab, present or missing |
| Clusters | a table, not rows: each added context with its hue and name, its read-only toggle, its login and remove; only the toggles and buttons act · then the kubeconfig's other contexts, each with add |
| Agent | auto-approve default · the shared repo, by URL and branch · the skills — core, the user's own, shared — each with its toggle · the routines — pause all, how many run at once, each routine and its triggers |
| Appearance | theme — Latte, Frappé, Macchiato, Mocha · UI font: IBM Plex Sans · mono font: IBM Plex Mono, JetBrains Mono, Lilex · three sizes: interface, editor, terminal |
| Preferences | attention thresholds — review waiting, due soon and approved unmerged in days, a failed run in minutes · the forge poll's interval and its stale threshold |
| Keymap | every action with its chords; a chord is rebound by pressing it |

Every change saves to the config file on the spot. No save button.
