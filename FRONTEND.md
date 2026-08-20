# Frontend rework — structure & slices

The backend rework is done (see REWORK.md): a frozen, feature-partitioned, tested
contract. The frontend is rebuilt on it, feature-first, mirroring the backend 1:1.
Design is pinned in the Excalidraw boards (Wiremind → Main) + the Catppuccin-Latte
mockup; this file is the code plan.

## Target `src/`

```
src/
  main.tsx
  app/            App (home|session|overview|review) + chrome + providers (useIpc, keys, theme)
  shared/         the frontend "core"
    ipc/          client.ts · events.ts · generated/  (ts-rs output from Rust)
    store/        index.ts composes feature slices
    ui/           Button Chip Tag Tab Panel Pill CIDot Icon Markdown ContextMenu Toasts
    lib/          layout · keys · keybindings · match · taskStatus · openExternal · goToSession · diff/ (ONE engine)
    styles/       tokens.css + themes/*  (Catppuccin, kept as-is)
  home/           Home · LiveSection · UpNext · NewTaskModal · home.slice
  sessions/       sessions.slice (session.ts pure logic) · SessionPicker (switch/close; no dock)
  workspace/      Workspace · WorkspacePane · WorkspaceLayout · TabStrip · SessionWorkspaces · panes
  editor/         CodeEditor · FileDiffEditor · setup/ · annotations/ (inline widget) · useAnnotations/Blame/DiffExpand
  files/          FilesPanel · tree · FileTreeMenu · search
  git/            GitPanel (Changes+commit box · Forge: MR·CI·threads·commits) · DiffView · CommitDiffView
  notes/          NotesPanel (local annotations + review mr threads) · MrThreads
  overview/       Overview (by kind) · TaskOverview (props·hours·body) · ReviewOverview (MR)
  agent/          AgentConsole (state / PTY / actions) · agentSend · prompts
  terminal/       TerminalConsole (Home scratch) · PtyTabBody · terminalHost · useAttachedHost
  approvals/      ConfirmModal · ops.ts · per-op renderers · approvals.slice
  notifications/  Toasts · NotificationCenter · notifications.slice
  command/        CommandPalette · useListNav · registry
  setup/          FirstRun (email→find_notion_user) · AuthModal · SettingsModal · TaskOpenWizard · AddRepoModal · repoPicker
```

Rules: features import downward from `shared/` only; each feature owns its
components + hooks + store slice + CSS.

## Decisions
- **ts-rs: yes** — IPC types generated from the Rust structs into `shared/ipc/generated/`.
- **store slices: yes** — the god-store splits (sessions / home / notifications / approvals / command-ui); tested `session.ts` reducers survive.
- **CSS: keep the Catppuccin token system** as the shared design layer; feature CSS co-located.

## Design decisions baked in
- Home = pure dashboard (no agent, no dock). Live fold/expand (repo→worktree→MR w/ CI+threads); Up Next 4-col table (review row: name = MR id+title, state = repo). `+ New` = explorer. Scratch terminal (deskless).
- Header = context pickers (Active Session/Repo/Worktree); no session dock — the Session picker switches/closes.
- Rail = spine: Home · (Ovw · Files · Git · Notes) · Cmd. 44px, icon-only, grouped by a hairline.
- Overview is a session MODE, by kind: task → ticket (properties·hours·body); review → the MR overview. Agent console persists across Code↔Overview; pop-out ready.
- **MR links always open GitLab** (browser), like CI. No in-app MR view for tasks.
- Notes → local annotations (inline CM widget) + review-only mr threads.
- **Approvals are a standalone attention modal** (a global overlay that demands a decision), NOT inline in the agent console.

## Dies / ledger payoff
desk (15 files), ptyTrace, task-MR view, session dock, OverviewView dispatcher, RepoSwitcher name clash.
Owed items from REWORK.md land here: watch_task_worktrees/file_changed removal, `register_repo {slug,localPath}`,
`MainRepo.url` gone, refresh contract (`flush_git_caches → invalidateDiff → refreshStatusFor`, driven by
`agent_activity` + button), B11 per-op renderers, N5 commit→push chaining, P9 base64 `write_pty`,
diff cache re-key `${worktreeId}/${path}`, `list_notion_users → find_notion_user`, dropped `notion.status` renderer.

## Slices (each leaves the app runnable; verified by tsc/vitest/tauri dev)
0. **Foundation** — shared/ (ipc client, ts-rs types, events, store scaffold), tokens/themes, ui primitives, app shell + view routing, get_config→FirstRun.
1. **Home (MVP)** — home.slice (get_home_snapshot, list_tasks, list_review_mrs), Live, Up Next, open task/explorer/review. *0+1 = the MVP.*
2. **Session core** — sessions slice, panes/tabs/layout, files panel, editor (+save), terminal (PTY, base64).
3. **Git & diff** — Changes/commit/commits, the one diff engine, diff modes, explicit-refresh wiring.
4. **Agent & approvals** — PTY agent console (state/term/actions), ConfirmModal per-op renderers + N5 chaining, notifications.
5. **Overview · Notes · Forge** — task + review Overview, inline annotations + mr threads, MR create/CI/approve, GitLab links.
6. **Palette · settings · wizards.** Command palette (⌘K), Preferences, first-run, add-repo, inline CM annotations. Pop-out agent window DROPPED by decision — the agent console stays docked in-app.

## Status
All six slices done on `rework/core-db`; each verified tsc + vite build + vitest. Agent console is in-app only (pop-out dropped). Not yet run against the real DB — end-to-end `pnpm tauri dev` test pending.
