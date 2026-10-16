# Concepts

## The board

The board is the first screen. `alt+h` opens it and goes back. It has three columns:

- **Live**: every session on disk.
- **Up next**: your tasks that no session works yet. Drag a row to put it in your own
  order. Groove does not tell the provider.
- **Review**: the MRs that wait for your review, from the forges.

The filter takes words, matched against the title, and `field:value` terms. The fields
are `status`, `priority`, `project`, `provider`, `kind` and `repo`.

## Sessions

A session is one unit of work. It holds its repos, worktrees, agent, terminals and log.
There are four kinds:

| Kind | What it is | How you open one |
|---|---|---|
| Task | Work on a ticket. It has a branch and usually ends in an MR. | Open a row in Up next. Groove sets the ticket to in progress. |
| Review | Someone else's MR, checked out to read and annotate. | Open a row in Review. |
| Explorer | Work with no ticket yet. | `alt+shift+n`, or "+ explorer" on the board. |
| Routine | The session of a routine you turned on. | Groove makes it. It shows under Routines on the rail. |

An explorer becomes a task when you file a ticket from it and adopt that ticket.

Close a session with `alt+shift+w`. Closing stops its agent and keeps everything on
disk. To remove a session, use Delete session in the palette.

## The window

- **The rail**, on the left: the open sessions and the feed of what they did.
- **The agent pane**: the agent of the selected session.
- **The workspace**: the Overview, Diff, Files and Kubernetes tabs.
- **The sidebar**: the files and lists of the open tab.

`alt+1` to `alt+4` move to each pane. The [keymap](reference/keymap.md) lists all keys.

## The palette

`alt+k` opens the palette. Every action is in it, grouped: Go, Session, Diff, Git, MR,
Agent, Skills, Notes, Task, Look, Config. Type `#` first to search the files of the
selected worktree. In a multi-step entry, Backspace on an empty field goes back one step.

## Repos and worktrees

Groove keeps one clone of each repo under `<worktree_root>/main`. It never writes in a
clone. Each session gets its own worktree of each repo it needs, cut from the clone
after a fetch:

```text
<worktree_root>/worktrees/<session>/<project>/<branch>
```

Add a repo to a session with Add repo in the palette. Two sessions never share a
worktree, so their changes never mix.

## Notes

A note is a comment on lines of code. Right-click the code in the diff or the editor and
choose **note**, or use "Add a note on the caret's line" in the palette. Groove keeps the
notes with the session. You can post them to the MR.

## The agent

Each session has one agent: Claude Code, started by Groove in the session. Groove gives it
tools to read and act on the session: the task, the diff, git, the MR and the clusters.
You set up nothing.

The agent cannot run `kubectl`, `helm` or `k9s` itself. It reads a cluster through Groove.

## Asks and approvals

A write the agent wants to do is an **ask**: a commit, a push, an MR, a comment, a cluster
attach. An ask waits for you. It shows on the session's row in the rail with **Approve**
and **Review**. Review opens a sheet where you read the action, then approve or refuse it.
`Esc` closes the sheet and decides nothing.

**Auto-approve** is a switch on each session, in the agent pane's bar and in the
palette. When it is on, Groove approves each ask as it arrives. The
`preferences.auto_approve_default` key sets the value a new session starts with.
Routines act only in sessions where auto-approve is on.
