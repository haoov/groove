# Skills and routines

## Skills

A skill is a procedure the agent follows, such as starting a task or fixing CI. The
agent pane's skills menu sends one to the agent. Groove has three sets:

| Set | Holds | Where it lives |
|---|---|---|
| `groove` | Groove's own procedures. Always on. | in the app |
| `user` | your own skills | `~/.config/groove/user-skills/` |
| your team's | the skills of the shared repo, one set for each plugin | the shared repo |

Settings › Agent lists every skill. Turn each of your own and each shared skill on or off
there. A shared skill starts off: each skill you turn on adds to what every agent reads.

To write a skill of your own, ask the agent for it. Saving it is an ask.

## The shared repo

Your team can share skills, facts and routines through one git repo. Name it in
Settings › Agent, by its URL and its branch. A private repo needs its SSH URL.

Groove keeps its own copy of that branch, and updates it at launch and when you read the
board again. To change the shared repo, add it to a session and open an MR, as for any
other change.

## Routines

A routine starts an agent on its own when an event happens, for example:

- CI fails on a worktree;
- a reviewer requests changes;
- someone asks you for a review;
- the first time Groove opens each day.

Settings › Agent lists the routines, yours and your team's. Turn each one on or off, and
each of its triggers. `preferences.routine_cap` sets how many routines run at once.

A routine acts only in a session where auto-approve is on. It never acts on the session
you are on, and it runs once on a session until you look at that session.
