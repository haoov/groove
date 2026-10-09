# Workspace

Review, edit and land the code. Surface: the session's work region, as
[../design.md](../design.md#workspace) draws it. Everything here follows the session's
selected worktree.

## Diff

**Two documents and an alignment.** A diff is the file before and the file after, each a
document, and the alignment between them, computed with `imara-diff`. Git gives the
content at a ref; the working copy comes from disk. Git's own diff output is never read:
no patch parser. Expanding a gap is a slice of a document already held.

**Two modes.** Base: against the merge-base with the base branch, or the MR's target in a
review. Working: uncommitted changes against HEAD. Untracked files are all additions. A
throttled background fetch keeps the base fresh.

**The whole change is one surface.** The file list is built from the numeric summary,
with no content. The files on screen hold their documents; the others give theirs up. A
file's rows fold under its own head row. The view — file, inline or split — the folds,
the find session and the scroll are the ui's own state, not commands.

**The syntax tree is whole-file, the query is windowed.** A document holds no colours;
the frame asks the tree what the rows on screen mean.

**Word diff** runs on one-for-one pairs of removed and added rows only.

**Capped by size.** Above 1 MiB a file is aligned but not coloured; above 2 MiB it is
listed as changed and not shown. A file git calls binary is listed only.

**Reading the worktree never writes it.** Every git command runs with
`GIT_OPTIONAL_LOCKS=0`.

**A change on disk reloads the diff.** A watcher on the selected worktree, `.git`
excluded, waits for 25 ms of quiet, then reloads the summary and the changed files as a
new [generation](../glossary.md#generation). The agent, the editor and a terminal all
write through the disk, so all three show.

**A file is marked read by the user**, never by scrolling past it. The marks belong to the
session.

**A shown commit is read-only.** `workspace.open_commit` shows one commit as the change
it made, read from the commit and its parent. `dispatch` drops every command that would
write while one stands, and `workspace.leave_commit` puts the working tree back.

**Blame** is read once a read of the file. A caret resting on a line for a quarter
second shows `Author, 5d ago · sha` at its end.

## Resources

**The list reads the server's Table**: the columns `kubectl get` shows, for every kind,
CRDs included. A list never waits on the cluster: its watchers run on the pool and send a
batch a frame at most.

**A watcher runs while the tab reads it.** The tab holds one reader over a watcher a
context and namespace in scope, one a context for a kind not namespaced. A change of
scope, kind or label lets go of them all and reads the new ones; a watcher left unread
stops thirty seconds later.

**A name word narrows in Groove, fuzzily — its characters in order; a `label=value` word
goes to the server**, as a watcher of its own. Both are typed in the list's find bar; the sidebar's search narrows the kinds.

**A status and a ready count wear the colour of how the object stands**: good, waiting,
failing, or done. A row whose status is done keeps its ready count faint. A word Groove does
not know stays plain.

## Annotations

**A note is the session's own**: a line range on a file's new side, its words, its
author and its status, kept in Groove's database. The notes list shows the session's
notes and the MR's threads as one list.

**One open note a line.** A note stands in the surface under the line it was left on,
with its buttons under it.

**What a note offers depends on what holds it.** A note of the session is written again,
resolved, deleted, or posted where the worktree has an MR; posting resolves the local
copy. A forge thread is replied to and resolved.

**A verdict carries every open note of the repo**: in one atomic call on GitHub, and a
note at a time on GitLab, each resolved as it lands.

## Editor

**One buffer a path, a worktree.** Each worktree keeps its open files in tab order;
switching worktree keeps them. Closing a worktree or deleting its session drops them.

**One preview tab.** A single click opens a file as the preview, and the next single
click replaces it. A double click, `workspace.keep_file` or the first edit keeps it.

**A path never leaves the worktree.** Every file operation resolves against the worktree
root and is refused when it escapes.

**The rules of editing:**

- **The buffer answers at once; what it implies follows.** A keystroke costs the rope
  edit and a shift of the tree's nodes. The parse and the alignment run in a job and land
  only if the buffer has not moved on.
- **A dirty buffer outranks the disk.** While the buffer owes the disk, a write under the
  worktree refreshes the summary and leaves the buffer alone. A save clears the debt.
- **The caret is a place in the document**, carried across a reopen and a change of view,
  clamped to a place that exists.
- **A read that finds the text the buffer holds leaves the buffer alone**, so a save costs
  neither the undo history nor the caret.
- **An undo takes back a typing run**, not a character. A motion or a newline ends the
  run.

**Search** walks the worktree in parallel, reports in batches, caps what it returns, and
stops when the next search starts. The path search covers the whole worktree, not only
the diff.

**The clipboard** is the desktop's, or Groove's own when there is no desktop.

## Terminal

**A session's terminals run the user's login shell in the session's own directory**, not
in a worktree. A tab holds one terminal, or several side by side. Closing, deleting or
adopting the session ends its terminals.

**Keys, a paste and the wheel go to the selected terminal as bytes**; nothing is parsed.

## Git

**The commit box commits the index**; stage and unstage are per file. Its menu holds
push, pull and discard all.

**A push goes to the branch's own name on origin**, and nowhere else. A pull is
fast-forward only. [../trace.md](../trace.md) follows a push end to end.

**From the agent, every one of these is an ask** unless auto-approve is on.

## Forge

**The host decides the forge**: GitLab or GitHub, behind one `Remote` enum. Tokens come
from `gh auth token` and `glab auth status`, held for the run, never stored.

**One MR a worktree at most.** It is created from the worktree's branch to its base
branch or the repo's default, with a footer linking the task. The commit box's message
titles it, and its other lines are the body; an empty box takes the task's title.

**One read brings the MR, its CI and its threads.**

**The poll runs only while the window has focus.** It asks about the selected worktree
until its forge has answered once, then about every worktree whose MR is open. A read
that fails marks the row *stale* and keeps what it showed. A focus gain asks about
everything again; a push forgets what was asked of that worktree. A host Groove cannot
read is skipped, not reported.

**A verdict — approve or request changes — is given in a review session only**, with the
commit box's words as its comment. GitLab gives a verdict no time of its own, so the MR's
`updatedAt` stands for it.
