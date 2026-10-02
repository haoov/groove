# Shared skills, knowledge and routines — design

What a team shares through Groove: the skills its agents follow, the facts they should know,
and the routines that act on their own. Designed, not built. It touches three capabilities —
Config holds the shared repo, Agent offers its skills and runs the routines, Sessions edits it
like any other repo — and it follows [architecture.md](architecture.md) like every other
feature.

## What it answers

- A skill one person writes reaches the whole team, reviewed like code.
- What one agent learns about the work reaches the others, without the user's own
  preferences.
- An event Groove sees can start an agent without the user, inside limits the user set once.

## The shared repo

**One git repo holds what the team shares.** Settings › Agent names it, by its URL and the
branch it follows. Groove keeps a copy of that branch of its own, under its data directory, apart from
the repo pool: nobody edits it, and the clone the user works in is never touched. The copy
moves to the branch's head when the app starts and when the board is read again; a copy that
cannot move keeps what it held and the feed says why, once. No repo named, nothing is shared.

**A private repo is named by its SSH URL.** Groove runs git with no prompt, so a URL that asks
for a password cannot be read; the refusal says to give the SSH one.

**The repo is a Claude Code plugin marketplace.** Its root holds
`.claude-plugin/marketplace.json`, which lists its plugins, then:

- `plugins/<name>/` — one plugin: its own `.claude-plugin/plugin.json`, named as its entry, and
  `skills/`, one directory a skill with its `SKILL.md`;
- `knowledge/` — one Markdown file a fact;
- `routines/` — one file a routine.

Groove reads only plugins that stand in the repo itself: an entry whose source is another repo,
or a path outside this one, is refused. The same repo works outside Groove: Claude Code adds it
as a marketplace by itself.

**A change to the repo is a task on it.** The repo is added to a session like any other: a
worktree, a branch, a commit, an MR, a review. The MR is the trust model: a
shared skill is text every teammate's agent obeys, so it is reviewed before it lands.

## Shared skills

**One namespace a plugin, beside `groove` and `user`.** A plugin's name is the namespace of its
skills: `platform:fix-ci`, `review:triage`. A plugin named `groove` or `user` is refused. A
skill keeps its name once it is shared, since skills and routines call it by that name: a new
domain is a new plugin, never a rename. The skills menu lists the team's enabled skills, under
their plugin, between the core ones and the user's own.

**Each team skill is enabled one by one.** Every enabled skill costs each session the words
that describe it, and fills the skills menu. A skill the user has not enabled is not offered to
the agent and not shown in the menu. A new skill in the repo starts off.

**Settings › Agent lists every skill, in three groups:**

- core (`groove`): each skill and what it does, always on — the app's own buttons send them;
- the user's own (`user`): each with a toggle, on when written, and a delete that asks first;
- shared, under each plugin: each with a toggle, off when it arrives.

Settings shows and switches; the agent writes a skill, as it does today.

**Groove passes only what is enabled.** A plugin directory offers every skill it holds, so the
agent is not given the copy: for each shared plugin with an enabled skill, Groove builds a
plugin of its own beside the core one, with that plugin's name and only its enabled skills,
linked from the copy.

**A namespace never overrides another.** A team skill and a user skill can have the same name;
each is reached by its own namespace.

**A skill is shared through the repo, not from Settings.** A user who wants one of theirs shared
adds the repo to a session and lands the skill as an MR, like any other change.

## Knowledge

**A fact worth sharing is promoted, not copied whole.** The agent's own memory stays the
user's: what it knows of the user and how they work is never shared. A fact about the work — a
system, a repo, a tool, a trap — can be promoted to `knowledge/`, through the same task and MR
as a skill. The `promote-fact` skill does it, checking the fact against its source first.

**Every fact says where it comes from and when.** A fact carries the date it was written and
the commit, file or ticket it was read from, so a reader can check it before acting on it. A
fact nobody can check is not promoted.

**The agent reads the facts like its memory.** The core prompt names `knowledge/` and its
index when the copy holds one; the agent reads it when the user asks or when it judges it
useful, opens the facts that look relevant, and checks one before it relies on it.

## Routines

**A routine is a trigger, a skill and a scope.** It is one file: in the shared repo's
`routines/` for the team's, in the user's config for their own. Two kinds:

- **Bound to a session:** the trigger is an event about one session, and the skill goes to
  that session's own agent, with what the event says as its arguments — CI failed on a
  worktree → `groove:fix-ci`; changes requested on an MR → `groove:fix-notes`; a review asked
  of the user → `groove:co-review` in its review session.
- **Standalone:** the work belongs to no session — check that every task has its properties
  set, say. The routine has a session of its own to run in.

**A standalone routine runs in a routine session.** A session of the kind `routine`, one per
standalone routine, made when the routine is switched on. It stands on the rail under its own
folded Routines group and never on the board. It holds no repo, worktree, diff or MR: only its
agent pane, which the user talks to as to any agent. Each run starts its agent afresh; the
runs before stay in its history, and the user can step in while one runs.

**What starts a routine:**

- its button, on the routine's row, always;
- the first time the app opens each day;
- an app event: CI failed, changes requested, a review asked of the user, an agent that
  finished, the tasks read again, a task moved to a status.

The file declares the triggers it answers to. Settings › Agent lists each routine with them,
and the user turns the daily one and each event on or off there.

**A routine acts without asking.** The approval moves from the run to the routine: a routine
declares its scope — the tools it may call and the writes it may make — and switching it on is
the user's approval of that scope. A run inside its scope waits on nobody.

**A call outside the scope fails; it does not wait.** The run ends there, and the feed says
which call it tried and why it was refused. Nothing is queued for the user.

**The scope holds for every tool the agent calls.** Groove's own tools go through its approval
queue, which lets a call in the scope pass and refuses the rest. The tools of another server —
Notion, say — never reach that queue, so the same scope is written as the agent's own
permissions when Groove launches it: Claude Code refuses what it does not allow.

**Some writes are never in a scope:** a merge, a force push, a delete, closing an MR or a task,
any write to a cluster. Those stay the user's, whatever the routine declares.

**The widest scope a bound routine can declare:** editing files in the session's worktrees,
committing, pushing to the session's own branch, and commenting on the forge. Opening or
updating an MR only when the routine names it. A standalone routine names each tool it may
call, of Groove or of another server.

**One run a routine and a session at a time.** A routine does not start again on a session
where its last run still runs, and runs on several sessions at once. Settings caps how many
agents run routines at the same time, five by default; a run past the cap waits its turn.

**Every run leaves a trail.** The feed shows each run: the trigger, the skill, what it did and
the commits it made. A user who was away reads what happened there.

**One switch stops them.** Each routine can be paused, and Settings pauses them all at once.

**A team routine is never on by default.** Routines in the shared repo run with the
credentials of the user who enables them, so each user enables each one, after reading its
scope.

**Routines run while the app runs.** Groove is a desktop app: an event that happens while it
is closed starts nothing, and the daily trigger fires at the first open of the day.

## Not in it

- No clock: a routine starts on an event, its button or the first open of the day.
- No chain: one trigger, one skill. Conditions, retries and steps are what CI does.
- No shared memory as a whole: only facts promoted one by one.
