# Shared skills, knowledge and routines — design

What a team shares through Groove: the skills its agents follow, the facts they should know,
and the routines that act on their own. Designed, not built. It touches three capabilities —
Config holds the shared repo, Agent offers its skills and runs the routines, Sessions edits it
like any other repo — and it follows [architecture.md](architecture.md) like every other
feature.

## What it answers

- A skill one person writes reaches the whole team, reviewed like code.
- What one agent learns about the platform reaches the others, without the user's own
  preferences.
- An event Groove sees can start an agent without the user, inside limits the user set once.

## The shared repo

**One git repo holds what the team shares.** Settings names it, by its URL and the branch it
follows. Groove clones it into the repo pool like any other repo and pulls it when the app
starts and when the board is read again. No repo named, nothing is shared.

**The repo is a Claude Code plugin.** Its root holds `.claude-plugin/plugin.json`, then three
directories:

- `skills/` — one directory a skill, each with its `SKILL.md`, as the core and user plugins
  hold them;
- `knowledge/` — one Markdown file a fact;
- `routines/` — one file a routine.

The same repo works outside Groove: Claude Code reads it as a plugin by itself.

**The app never writes in the clone.** A change to the repo is a task on it: a worktree, a
branch, a commit, an MR, a review. The clone only fast-forwards. The MR is the trust model: a
shared skill is text every teammate's agent obeys, so it is reviewed before it lands.

## Shared skills

**A third namespace beside `groove` and `user`, named by the repo.** Its `plugin.json` names
the plugin, and that name is the namespace: `wiremind:fix-ci`. A repo named `groove` or `user`
is refused. The skills menu lists the team's enabled skills between the core ones and the
user's own.

**Each team skill is enabled one by one.** Every enabled skill costs each session the words
that describe it, and fills the skills menu. Settings lists the team's skills, each with a
toggle; a skill the user has not enabled is not offered to the agent and not shown in the
menu. A new skill in the repo starts off, and Settings says how many arrived since the user
last looked.

**Groove passes only what is enabled.** A plugin directory offers every skill it holds, so the
agent is not given the clone: Groove builds a plugin of its own beside the core one, with the
repo's name and only the enabled skills, linked from the clone.

**A namespace never overrides another.** A team skill and a user skill can have the same name;
each is reached by its own namespace.

**A user skill can be published.** Its menu offers "share with the team": Groove opens a task
on the shared repo with the skill copied into `skills/`, and the user lands it as an MR. Once
it is merged, Groove offers to delete the user's own copy.

## Knowledge

**A fact worth sharing is promoted, not copied whole.** The agent's own memory stays the
user's: what it knows of the user and how they work is never shared. A fact about the platform
— where a version is pinned, which environment still runs which ingress, which token a service
shares — can be promoted to `knowledge/`, through the same task and MR as a skill.

**Every fact says where it comes from and when.** A fact carries the date it was written and
the commit, file or ticket it was read from, so a reader can check it before acting on it. A
fact nobody can check is not promoted.

**The agent reads the facts like its memory.** The core prompt names `knowledge/` and its
index; the agent reads a fact when it looks relevant, and checks it before it relies on it.

## Routines

**A routine is a trigger and a skill.** When the event fires, Groove sends the skill to the
agent of the session the event is about, with what the event says as its arguments:

- CI failed on a worktree → `fix-ci`;
- a review is asked of the user → `co-review` in its review session;
- changes requested on an MR → `fix-notes`.

The triggers are events Groove already sees: CI failed, changes requested, a review asked of
the user, an agent that finished, a task moved to a status. A button can also start a routine.

**A routine acts without asking.** The approval moves from the run to the routine: a routine
declares its scope — the tools it may call and the writes it may make — and enabling it is the
user's approval of that scope. A run inside its scope waits on nobody.

**A call outside the scope fails; it does not wait.** The run ends there, and the feed says
which call it tried and why it was refused. Nothing is queued for the user.

**Some writes are never in a scope:** a merge, a force push, a delete, closing an MR or a task,
any write to a cluster. Those stay the user's, whatever the routine declares.

**The widest scope a routine can declare:** editing files in the session's worktrees,
committing, pushing to the session's own branch, and commenting on the forge. Opening or
updating an MR only when the routine names it.

**One run at a time.** A routine does not start again on a session where its last run still
runs: an event that fires three times in an hour starts one agent, not three.

**Every run leaves a trail.** The feed shows each run: the trigger, the skill, what it did and
the commits it made. A user who was away reads what happened there.

**One switch stops them.** Each routine can be paused, and Settings pauses them all at once.

**A team routine is never on by default.** Routines in the shared repo run with the
credentials of the user who enables them, so each user enables each one, after reading its
scope.

**Routines run while the app runs.** Groove is a desktop app: an event that happens while it
is closed starts nothing.

## Not in it

- No schedule: a routine starts on an event or a button, never on a clock.
- No chain: one trigger, one skill. Conditions, retries and steps are what CI does.
- No shared memory as a whole: only facts promoted one by one.
