# Design

Three surfaces, three scopes, and settings beside them. Information never leaks
upward: what belongs in the session does not reach the rail; what belongs on the
board does not reach the rail.

| Surface | Scope | Holds | Capability |
|---|---|---|---|
| Board | large | every task, live or not, with its repos and MRs | Tasks |
| Rail | mid | the sessions actually open, and what their agents do | Agent › Multi-agent |
| Session | small | one session, in full | Sessions, Workspace |
| Settings | — | setup, providers, appearance, preferences | Config |

## Rules

- **Colour means attention. Motion means activity.** Nothing else uses either
  channel. If a thing is coloured, the user is needed. If it moves, an agent works.
- **A fact clears when the world changes, never when the user looks at it.** No
  dismiss.
- **Suggest, never auto-send.** The action of a state is the skill that fixes it,
  shown as a button. Groove never runs it by itself.
- **Everything is reachable from the keyboard.** The command palette is the front
  door to every action in every capability. The mouse is a shortcut.

## Board — large scope

The complete lists: **Live**, **Up next**, **Review**. Form and design open.

Each item has two forms:

| Form | Shows |
|---|---|
| folded | title, type, repo count |
| expanded | the repo list, each with its MRs and an external link |

Quick links to the externals: the provider and the forge.

Task actions, on the item: open in provider · finish · delete · delete locally.

Delivery per worktree appears here as counts, never as a verdict.

- [ ] Sections, sorting and filtering.

## Rail — mid scope

Only the sessions actually open, in the order opened. Never re-sorted.

Each row:

1. **Type icon** — task, review, explorer.
2. **Title.**
3. **Agent status** — `setting up`, `working`, `committed 2 · 5 files`, `idle`,
   `exited`, `error`. One line, truncated, never wrapped.
4. **Agent action, with buttons when it needs one** — `asks to push to main`
   **Approve** **Review**.
5. **Relative time, right-aligned** — how long the agent has waited, or since it
   finished.

```
 ▤  Board
 ─────────────────────────────────────────
 ⚑  TASK-51  auth refresh                2m
    asks to push to main         [Approve] [Review]
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

An ask is coloured and offers *Approve* and *Review*. A working status moves,
monochrome.

Opening a session starts its agent. Closing a row ends the agent and removes the row.
The session stays as it is on disk.

## Session — small scope

One session in full. Three regions, layout open:

| Region | Holds |
|---|---|
| **Agent pane** | the terminal with the discussion — Sessions › Agent session |
| **Session overview** | body, properties, the repo and worktree list — the selector for the session — Sessions › Overview |
| **Work** | diff / annotations / editor / terminals / git / forge / timeline — Workspace |

Delivery per worktree — CI, MR state, review notes, land, resolve, close worktree —
lives on the worktree list in the overview. Closing the task is offered there when
every worktree is merged or closed.

- [ ] Layout of the three regions: splits, tabs, or both.
- [ ] **Diff first.** A session with changes opens on the diff of the worktree the
      agent last touched; the editor is the drill-down.
- [ ] **Timeline** placement within Work.
- [ ] **Review with context.** An ask opens the thing itself: a commit is its diff
      and message, an MR its rendered description, a task update before and after.
      Editable in place, then approve.
- [ ] **Notes.** Where annotations live relative to the diff and the timeline.

## Settings

One surface for Config: setup, providers, appearance, preferences. Reached from the
palette and the board. Form open.

- [ ] Modal or page.
