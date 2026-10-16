# First run

The first run takes three steps: install the tools, sign in, and connect a task source.

## The config file

Groove keeps its settings in `~/.config/groove/config.json`, or in
`$XDG_CONFIG_HOME/groove/config.json`. At the first launch, Groove creates it with
`~/groove` as the worktree root, the folder that holds your repos:

- the clone of each repo goes under `~/groove/main`;
- the worktrees of your sessions go under `~/groove/worktrees`.

To use another folder, change `git.worktree_root` in the file. Settings writes every
change to the file at once. You do not restart Groove.

Groove keeps its state in `~/.local/share/groove/app.db` and its cache in
`~/.cache/groove`. The `XDG_DATA_HOME` and `XDG_CACHE_HOME` variables move them.

## 1. Install the tools

Open Settings with `alt+,`. When a required tool is missing or Claude is signed out,
Groove opens the **Setup** section on its own.

| Tool | Required | Groove uses it for |
|---|---|---|
| `git` | yes | worktrees, diffs, commits |
| `claude` | yes | the agent of each session |
| `gh` | no | GitHub pull requests and CI |
| `glab` | no | GitLab merge requests and CI |
| `kubectl` | no | the agent's cluster reads |
| `jq` | no | filters on the agent's cluster reads |
| `curl` | no | the agent's status on the rail |

Install what is missing, then click **check again**. Groove finds a tool on the `PATH`
of your login shell. When it does not find one you installed, read
[Troubleshooting](troubleshooting.md).

## 2. Sign in

- **Claude**: in Setup, click **sign in** next to Claude login. It runs
  `claude auth login` in a terminal inside Setup.
- **GitHub**: run `gh auth login`. Groove reads the token from `gh` and never stores it.
- **GitLab**: run `glab auth login --hostname <your host>`. Groove reads the token from
  `glab`.

Settings › Providers › Forge tokens shows if Groove finds a token for each one.

## 3. Connect a task source

A task source gives Groove your tickets. Open Settings › Providers and turn one on. You
can keep both on, but at least one stays on.

### GitHub issues

Turn on GitHub and keep the host `github.com`, or type your GitHub Enterprise host.
Groove takes the token from `gh auth token`. Then map the status and priority values of
your issues.

### Notion

Turn on Notion and give it three values:

- **Token**: the secret of a Notion integration that can read your task database.
- **Database**: the id of the task database. It is the 32-character part of the
  database URL.
- **User**: your Notion user id. Groove lists only the tasks assigned to you.

Then map the properties of the database. Groove offers only the properties of the right
type: a date for start and due, a number for estimate and logged time, a person for the
assignee, a relation for the sprint. Map the assignee and the sprint first: until you
map both, Groove lists no task.

Under status, give the database values that mean ready, in progress and done. Under
priority, give the values that mean high, medium and low.

## Next

Your tasks show on the board, in the Up next column. Read the
[concepts](concepts.md) before you open one.
