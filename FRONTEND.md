# Frontend architecture

The frontend is the ORIGINAL app's code, reworked in place (2026-08-20): feature-first
layout, ts-rs types, a sliced store, the REWORK.md ledger paid off, and the mockup-phase
UX decisions applied — same visual identity, same features. History: a from-scratch
rebuild was attempted first and rejected; the correction ported the original code into
this structure and then reworked it. REWORK.md is the backend's ledger.

## Layout

```
src/
  main.tsx        fonts (bundled Plex Sans + Lilex/Plex Mono) · global.css · feature css
  app/            App (composition root) · chrome/ (Header+pickers, ActivityRail,
                  StatusBar, WindowControls, ResizeHandles) · providers/ (useIpc, useKeybindings)
  shared/         the frontend core — features import DOWN from here only
    ipc/          ipc.ts (THE type surface) · generated/ (ts-rs, via `pnpm gen:types`)
                  invoke.ts (timing wrapper) · events.ts · ops.ts (both mirror Rust; tests guard ops)
    store/        index.ts (barrel: useStore, useSession, SessionIdContext, accessors)
                  types.ts (slice interfaces; AppState = intersection) · session.ts (pure reducers + tests)
                  slices/ (ui, home, sessions+machinery, agent, confirmations, config,
                  keybindings, notifications)
    lib/          terminalHost · ptyRegistry · endSession · refreshSession · gitChain ·
                  agentSend · panes · goToSession · diffModes · workspace · taskStatus ·
                  keybindings · keys · layout · match · mr · icons · theme · notionUser ·
                  openExternal · useListNav · useAttachedHost · word-diff · diffGaps · cm langs
    ui/           Markdown · ContextMenu · StatBadge · propertyControls
    styles/       global.css (imports ↓) · tokens.css · themes/ · ui.css (shared bases:
                  .btn-*, .ctx-*, .composer-*, .nb-* markdown, .spin)
  home/ sessions/ workspace/ files/ git/ notes/ editor/ overview/ agent/ terminal/
  approvals/ notifications/ command/ setup/    — each owns components + css
```

## Rules

- **Types**: features import `shared/ipc/ipc` only — never `generated/`. ipc.ts
  re-exports the generated types and rebuilds the deliberate narrowings (DiffLine,
  Annotation.status, UiConfig.theme). `pnpm gen:types` regenerates; the Rust
  mirror tests (`approvals::ops::tests`) guard the op names in both directions.
  The `Task` DTO stays named Task (= generated TaskView): "Session" already names
  the workspace concept in the store.
- **Dependencies**: features import from `shared/` and themselves. Declared
  exceptions: `app/ → *` (composition root); `workspace/ → files|git|notes|editor|
  overview|terminal` (tab/sidebar host); `git/ → editor/` (data → renderer);
  hosts may import leaf features (`overview → notes/MrThreadsSection`).
- **Store**: one zustand store from slices; every consumer imports the barrel.
  Slices are full-state creators (cross-slice writes go through set/get). The
  `buildView` WeakMap and bound-action caches are perf invariants — a session
  object identity changes only on real change.
- **Refresh contract**: `shared/lib/refreshSession` = flush_git_caches →
  invalidateDiff → refreshStatusFor (+ refreshHome off-workspace). Driven by
  agent activity (throttled while working, immediate on idle) and the sidebar
  refresh button. There is no filesystem watcher.
- **PTY**: base64 both directions (`pty_output.b64`, `write_pty.dataB64`).
  IPC timing: `localStorage.setItem('wb.ipcTiming','1')` logs every call.
- **Worktree-centric**: `activeWorktreeId` is the git-ops target;
  `activeRepoId` derives from it. Diff/blame/expansion caches key on
  `${worktreeId}/${path}`; diff summary entries pair by worktree branch.

## UX decisions (2026-08, agreed with the owner)

- Identity: Catppuccin **Latte default** (config wins) · IBM Plex Sans UI ·
  **Lilex** → IBM Plex Mono code — all bundled.
- Home = pure dashboard: **no desk** (the scratch terminal owns its PTY under
  `__scratch__`); Live entries fold/expand with aggregate chips and two-click
  finish/delete/discard; Up next is one table (code · name · state · owner)
  with a filter and approved reviews soft-hidden.
- **Overview is a session MODE** (`workspaceMode`), not a tab: fresh sessions
  land on it; any tab open flips to code; the rail is the spine
  (Home · Overview · Files · Git · Notes · ⌘K).
- **Header pickers** (Session · Repo · Worktree chips) replaced the SessionDock;
  Alt+S opens the session picker; the notification feed lives in the bell's
  popover (Ctrl+N).
- **MR links always open the forge** — no in-app MR tab for tasks; a review's
  MR overview lives in its session Overview. Notes panel = annotations + (for
  reviews) the MR discussion threads.
- Approvals stay a standalone modal (ConfirmModal); **Commit & Push** posts the
  push only after the commit's confirmation resolves approved (N5).
- Pop-out agent window: dropped by decision.
