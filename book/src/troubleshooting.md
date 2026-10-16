# Troubleshooting

## Groove does not find a tool

Setup shows a tool as missing, but it works in your terminal.

Groove runs your login shell once at launch to read its `PATH`, then gives that `PATH` to
every tool it runs. This matters on macOS and from a desktop launcher, where an app does
not get the `PATH` of your terminal.

- Set the `PATH` in a file your login shell reads, such as `~/.zprofile`, `~/.zshrc` or
  `~/.bash_profile`.
- Make sure the shell starts in less than 5 seconds. Groove keeps the `PATH` it inherited
  when the shell is slower, fails, or when `$SHELL` is not set.
- Restart Groove, then click **check again** in Setup.

Groove also looks for `claude` in `~/.local/bin`, `/usr/local/bin` and `/usr/bin`.

## GitHub or GitLab says there is no token

The error reads `no token for <host>`. Groove does not keep forge tokens. It asks `gh`
or `glab` for one each time it needs it.

- GitHub: run `gh auth login --hostname <host>`.
- GitLab: run `glab auth login --hostname <host>`.

Then check Settings › Providers › Forge tokens. It shows each tool as present, missing,
or "no cli" when the tool itself is not installed.

## Notion lists no task

Map the assignee and the sprint in Settings › Providers. Until both are mapped, Notion
lists nothing. Check also that the integration has access to the database.

## A cluster is not in Settings › Clusters

Groove reads the kubeconfig at launch only. Restart Groove after you add the context.
When `$KUBECONFIG` is set, Groove reads only the files it names.

## macOS blocks the app after an update

Gatekeeper puts the quarantine flag back on each new version. Clear it again, as
[Install](install.md) shows.

## Report a bug

Open an [issue](https://github.com/haoov/groove/issues) with the error from the feed on
the rail.
