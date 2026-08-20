import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Search, Compass, LayoutGrid, PanelsTopLeft, LayoutPanelLeft, PanelRight, SquareTerminal,
  GitBranch, GitCommitVertical, ArrowUpFromLine, ArrowDownToLine, GitPullRequestArrow,
  ListTodo, PauseCircle, RefreshCw, Circle, Settings, Palette, Keyboard, type LucideIcon,
} from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { ensureTerminalTab } from '../shared/lib/panes';
import { DIFF_MODES } from '../shared/lib/diffModes';
import { Highlighted, matchRanges } from '../shared/lib/match';
import { THEMES, DEFAULT_THEME } from '../shared/ipc/ipc';
import { chordLabel } from '../shared/lib/keys';
import type { CommandId, Keymap } from '../shared/lib/keybindings';

interface Command {
  id: string;
  label: string;
  group: string;
  action: () => void | Promise<void>;
  /** Shortcut hint shown on the right of the row. */
  shortcut?: string;
}

/** First bound chord for a command, rendered as e.g. "Alt+Shift+E". */
function shortcutFor(keymap: Keymap, id: CommandId): string | undefined {
  const c = keymap[id]?.[0];
  return c ? chordLabel(c) : undefined;
}

const ICON_BY_ID: Record<string, LucideIcon> = {
  'nav-tasks-board': LayoutGrid,
  'nav-workspace': PanelsTopLeft,
  'mode-workspace': LayoutPanelLeft,
  'toggle-agent': PanelRight,
  'toggle-terminal': SquareTerminal,
  'git-commit': GitCommitVertical,
  'git-push': ArrowUpFromLine,
  'git-pull': ArrowDownToLine,
  'git-rebase': GitPullRequestArrow,
  'task-pause': PauseCircle,
  'task-sync': RefreshCw,
  'toggle-vim': Keyboard,
};
const ICON_BY_GROUP: Record<string, LucideIcon> = {
  Navigation: Compass,
  Workspace: LayoutPanelLeft,
  Git: GitBranch,
  Task: ListTodo,
  Editor: Keyboard,
  Preferences: Settings,
  Theme: Palette,
};
const cmdIcon = (c: Command): LucideIcon => ICON_BY_ID[c.id] ?? ICON_BY_GROUP[c.group] ?? Circle;

export function CommandPalette() {
  const commandPaletteOpen = useStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useStore((s) => s.setCommandPaletteOpen);
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  // Which repo the user is actually in. Git commands used to take the session's
  // FIRST active worktree, so on a multi-repo task they hit an arbitrary repo.
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const activeRepos = useSession((s) => s.activeRepos);
  const openTab = useSession((s) => s.openTab);
  const setDiffMode = useSession((s) => s.setDiffMode);
  const setLastError = useStore((s) => s.setLastError);
  const setView = useStore((s) => s.setView);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  const setTheme = useStore((s) => s.setTheme);
  const activeTheme = useStore((s) => s.config?.ui.theme ?? DEFAULT_THEME);
  const keymap = useStore((s) => s.keymap);
  const vimMode = useStore((s) => s.vimMode);
  const setVimMode = useStore((s) => s.setVimMode);

  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => {
    setCommandPaletteOpen(false);
    setQuery('');
    setSelected(0);
  }, [setCommandPaletteOpen]);

  // Opening is handled by the global keymap; close on Esc while open.
  useEffect(() => {
    if (!commandPaletteOpen) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [commandPaletteOpen, close]);

  useEffect(() => {
    if (commandPaletteOpen) {
      setQuery('');
      setSelected(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [commandPaletteOpen]);

  const buildCommands = (): Command[] => {
    // The focused repo's worktree, falling back to the only sensible default.
    const wt = activeWorktrees.find((w) => w.id === activeWorktreeId)
      ?? activeWorktrees.find((w) => w.repo_id === activeRepoId)
      ?? activeWorktrees[0];
    // Named in the label so a git command can never act on a repo you did not mean.
    const scope = activeRepos.find((r) => r.id === wt?.repo_id)?.project ?? null;
    const inRepo = (label: string) => (scope ? `${label} — ${scope}` : label);
    const cmds: Command[] = [];

    // Navigation
    cmds.push({
      id: 'nav-tasks-board',
      label: 'Go to Home',
      group: 'Navigation',
      shortcut: shortcutFor(keymap, 'view.tasks'),
      action: () => { setView('home'); close(); },
    });

    if (activeTask) {
      cmds.push({
        id: 'nav-workspace',
        label: 'Go to Workspace',
        group: 'Navigation',
        action: () => { setView('workspace'); close(); },
      });
      cmds.push({
        id: 'mode-overview',
        label: 'View: Task overview',
        group: 'Workspace',
        action: () => {
          setView('workspace');
          openTab({ repoId: '', filePath: '', view: 'diff', kind: 'overview' });
          close();
        },
      });
      for (const m of DIFF_MODES) {
        cmds.push({
          id: `diff-mode-${m.id}`,
          label: `Diff base: ${m.title}`,
          group: 'Workspace',
          action: () => { setDiffMode(m.id); close(); },
        });
      }
      cmds.push({
        id: 'toggle-agent',
        label: 'Agent: Open / focus',
        group: 'Workspace',
        shortcut: shortcutFor(keymap, 'agent.console'),
        action: () => { close(); useStore.getState().requestConsoleFocus(); },
      });
      cmds.push({
        id: 'toggle-terminal',
        label: 'Terminal: Open / focus',
        group: 'Workspace',
        shortcut: shortcutFor(keymap, 'workspace.toggleTerminal'),
        action: () => { close(); ensureTerminalTab(); },
      });
      cmds.push({
        id: 'terminal-new',
        label: 'Terminal: New',
        group: 'Workspace',
        action: () => { close(); ensureTerminalTab({ fresh: true }); },
      });

      if (wt) {
        cmds.push({
          id: 'git-commit',
          label: inRepo('Git: Commit changes…'),
          group: 'Git',
          action: () => {
            // Focus the sidebar's commit composer (wry's window.prompt returns
            // null, so a prompt-based flow was a silent no-op).
            close();
            useStore.getState().requestCommitFocus();
          },
        });
        cmds.push({
          id: 'git-push',
          label: inRepo('Git: Push'),
          group: 'Git',
          action: async () => {
            close();
            try {
              await invoke('push', { worktreeId: wt.id });
            } catch (e) {
              setLastError(String(e));
            }
          },
        });
        cmds.push({
          id: 'git-pull',
          label: inRepo('Git: Pull'),
          group: 'Git',
          action: async () => {
            close();
            try {
              await invoke('pull', { worktreeId: wt.id });
            } catch (e) {
              setLastError(String(e));
            }
          },
        });
        cmds.push({
          id: 'git-create-mr',
          label: inRepo('Git: Create merge request…'),
          group: 'Git',
          action: async () => {
            close();
            try {
              // Opens the confirmation with everything but the text pre-filled —
              // the same path as the sidebar's create-MR action.
              await invoke('create_mr', { worktreeId: wt.id });
            } catch (e) {
              setLastError(String(e));
            }
          },
        });
        cmds.push({
          id: 'git-rebase',
          label: inRepo('Git: Rebase on main'),
          group: 'Git',
          action: async () => {
            close();
            try {
              await invoke('rebase_on_main', { worktreeId: wt.id });
            } catch (e) {
              setLastError(String(e));
            }
          },
        });
      }

      cmds.push({
        id: 'task-pause',
        label: 'Pause task',
        group: 'Task',
        action: async () => {
          close();
          try {
            await invoke('pause_task', { shortId: activeTask.short_id });
          } catch (e) {
            setLastError(String(e));
          }
        },
      });
      cmds.push({
        id: 'task-sync',
        label: 'Sync task from Notion',
        group: 'Task',
        action: async () => {
          close();
          try {
            await invoke('sync_task', { shortId: activeTask.short_id });
          } catch (e) {
            setLastError(String(e));
          }
        },
      });
    }

    // Editor — always available
    cmds.push({
      id: 'toggle-vim',
      label: vimMode ? 'Disable Vim mode' : 'Enable Vim mode',
      group: 'Editor',
      shortcut: shortcutFor(keymap, 'editor.toggleVim'),
      action: () => { setVimMode(!vimMode); close(); },
    });

    // Preferences — always available
    cmds.push({
      id: 'open-settings',
      label: 'Open Settings…',
      group: 'Preferences',
      shortcut: shortcutFor(keymap, 'settings.open'),
      action: () => { setSettingsOpen(true); close(); },
    });
    for (const t of THEMES) {
      cmds.push({
        id: `theme-${t.id}`,
        label: `Theme: ${t.label}${activeTheme === t.id ? '  ✓' : ''}`,
        group: 'Theme',
        action: () => { setTheme(t.id); close(); },
      });
    }

    return cmds;
  };

  const commands = buildCommands();

  // Fuzzy, via the same matcher the file finder and repo picker use: a substring
  // filter meant "gcm" found nothing and every command had to be typed in full.
  // Matched against "group label" so "git push" and "gitpush" both land.
  const filtered = query.trim()
    ? commands.filter((c) => matchRanges(query, `${c.group} ${c.label}`) !== null)
    : commands;

  // Group display
  const groups = filtered.reduce<Record<string, Command[]>>((acc, cmd) => {
    (acc[cmd.group] ??= []).push(cmd);
    return acc;
  }, {});

  const flatFiltered = Object.values(groups).flat();
  const clampedSelected = Math.min(selected, flatFiltered.length - 1);

  // Keep the arrow-selected row scrolled into view.
  useEffect(() => {
    listRef.current?.querySelector('.selected')?.scrollIntoView({ block: 'nearest' });
  }, [clampedSelected]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, flatFiltered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      flatFiltered[clampedSelected]?.action();
    }
  };

  if (!commandPaletteOpen) return null;

  return (
    <div className="palette-overlay" onClick={close}>
      <div className="palette-modal" onClick={(e) => e.stopPropagation()}>
        <div className="palette-input-row">
          <Search className="palette-search-icon" size={16} strokeWidth={2} />
          <input
            ref={inputRef}
            className="palette-input"
            placeholder="Type a command…"
            value={query}
            onChange={(e) => { setQuery(e.target.value); setSelected(0); }}
            onKeyDown={onKeyDown}
          />
        </div>
        <div className="palette-list" ref={listRef}>
          {filtered.length === 0 ? (
            <div className="palette-empty">No commands match</div>
          ) : (
            Object.entries(groups).map(([group, cmds]) => (
              <div key={group} className="palette-group">
                <div className="palette-group-label">{group}</div>
                {cmds.map((cmd) => {
                  const idx = flatFiltered.indexOf(cmd);
                  const Icon = cmdIcon(cmd);
                  return (
                    <button
                      key={cmd.id}
                      className={`palette-item ${idx === clampedSelected ? 'selected' : ''}`}
                      onClick={() => cmd.action()}
                      onMouseEnter={() => setSelected(idx)}
                    >
                      <Icon className="palette-item-icon" size={15} strokeWidth={1.75} />
                      <span className="palette-item-label"><Highlighted text={cmd.label} ranges={matchRanges(query, cmd.label)} /></span>
                      {cmd.shortcut && <span className="palette-item-shortcut">{cmd.shortcut}</span>}
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>
        <div className="palette-footer">
          <span className="palette-footer-count">{flatFiltered.length} {flatFiltered.length === 1 ? 'command' : 'commands'}</span>
          <span className="palette-footer-hints">
            <kbd>↑↓</kbd> navigate <kbd>⏎</kbd> run <kbd>esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  );
}
