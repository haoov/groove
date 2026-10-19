# Troubleshooting

## Groove does not find a tool

Setup shows a tool as missing, but it works in your terminal.

An app started from the Dock or a desktop launcher does not get the `PATH` of your
terminal. So at launch, Groove asks your login shell for its `PATH` and gives it to every
tool it runs. It also adds the Homebrew folders on macOS, `/opt/homebrew/bin` and
`/usr/local/bin`, and looks for `claude` in `~/.local/bin`.

- Set the `PATH` in a file your login shell reads, such as `~/.zprofile`, `~/.zshrc` or
  `~/.bash_profile`.
- Make sure the shell starts in less than 5 seconds. Groove keeps the `PATH` it inherited
  when the shell is slower or fails, or when `$SHELL` is not set.
- Restart Groove, then click **check again** in Setup.

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

When Settings › Clusters says "could not read the kubeconfig", the words after it give the
cause.

## The shared repo does not update

The feed says why. A private repo needs its SSH URL: Groove runs git with no password
prompt.

## macOS blocks the app after an update

Gatekeeper puts the quarantine flag back on each new version. Clear it again, as
[Install](install.md) shows.

## Report a bug

Open an [issue](https://github.com/haoov/groove/issues) with the error from the feed on
the rail.
