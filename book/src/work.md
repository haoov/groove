# Work a task

This page follows one task from the board to the merged MR.

## 1. Open the task

On the board, open a row in **Up next**. Groove makes a task session and sets the ticket
to in progress at its source.

The **Overview** tab (`alt+shift+1`) shows the task's properties, its repos, the clusters
it holds and the ticket body.

## 2. Add the repos

Open the palette (`alt+k`) and choose **Add repo**. Groove cuts a worktree of that repo
on the task's branch. Repeat for each repo the task needs. To get a repo Groove does not
have yet, choose **Clone from a URL…**.

Each repo shows as a row on the Overview: its branch, its git status and its MR. The
button on a row sends the agent the skill that fits the row's state, such as saving the
work or fixing CI.

## 3. Work with the agent

The agent pane (`alt+2`) is a Claude Code terminal, started in the session. Ask it for
the work in plain words. It reads the task, the repos and the diff through Groove.

Each write it wants to do waits for you as an [ask](concepts.md#asks-and-approvals).
Approve it from the rail, or turn on auto-approve for the session.

The agent pane's bar holds the skills menu, **reload** and the auto-approve switch.

## 4. Review the change

The **Diff** tab (`alt+shift+2`) shows the whole change as one stream, file after file.

- It compares the branch with its base branch, or only the changes you have not
  committed yet.
- It shows the change inline or split. The palette switches the view.
- You can type in the new side of a file, as in the editor.
- `ctrl+f` finds text in the stream.

The **Files** tab (`alt+shift+3`) is the editor. `ctrl+shift+p` opens a file by its path,
and `ctrl+shift+f` searches the text of every file. In the palette, `#` finds a file by
its name.

## 5. Leave notes

Right-click a line and choose **note**. Pick a label, such as `issue` or `suggestion`,
type the words and save. The sidebar lists the notes of the session with the threads of
the MR.

A note stays in Groove until you post it from its card. You can also ask the agent to
fix the open notes.

## 6. Commit, push and open the MR

The sidebar of the Diff tab lists the changed files, with the **commit box** under them.

1. Stage the files: Space on a file, or right-click it. `alt+c` goes to the commit box.
2. Type the commit message, then `ctrl+enter` commits.
3. The button of the commit box always offers the next step: commit, push, pull, or open
   the MR. The caret beside it holds the rest: update or close the MR, discard every
   change.

The first line of the commit box titles a new MR, and the other lines are its body. An
empty box takes the task's title. Groove opens the MR from the task's branch to its base
branch, with a link to the task.

Groove reads the MR, its CI and its threads every minute while the window has focus.

## 7. Finish the task

When no MR of the task is still open, choose **Finish the task** in the palette or on the Overview.
Groove sets the ticket to done at its source, then removes the session: its agent and its
worktrees. When a worktree holds work you did not commit or push, Groove stops and
removes nothing.

## Review someone else's MR

Open a row in the **Review** column. Groove makes a review session with the MR checked
out. Read the diff, leave notes, then in the palette choose **Approve the MR** or
**Request changes**. The words in the commit box go with your verdict as its comment,
and your open notes are posted with it.
