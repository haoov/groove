import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '../shared/ipc/invoke';
import {
  Search, PanelsTopLeft, Compass, FileCode, Plus, ArrowUpFromLine, ArrowDownToLine,
  GitPullRequestArrow, ChevronsUp, PauseCircle, RefreshCw, Palette, type LucideIcon,
} from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { ensureTerminalTab } from '../shared/lib/panes';
import { DIFF_MODES } from '../shared/lib/diffModes';
import { Highlighted, matchRanges } from '../shared/lib/match';
import { THEMES, DEFAULT_THEME } from '../shared/ipc/ipc';
import { commandRows, runCommand } from '../shared/lib/commands';

interface Command {
  id: string;
  label: string;
  group: string;
  icon: LucideIcon;
  action: () => void | Promise<void>;
  /** Shortcut hint shown on the right of the row. */
  shortcut?: string;
}

export function CommandPalette() {
  const commandPaletteOpen = useStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useStore((s) => s.setCommandPaletteOpen);
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  // The focused repo. A git command must never act on an arbitrary worktree of the session.
  const activeRepoId = useSession((s) => s.activeRepoId);
  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const activeRepos = useSession((s) => s.activeRepos);
  const setWorkspaceMode = useSession((s) => s.setWorkspaceMode);
  const setDiffMode = useSession((s) => s.setDiffMode);
  const setLastError = useStore((s) => s.setLastError);
  const setView = useStore((s) => s.setView);
  const setTheme = useStore((s) => s.setTheme);
  const activeTheme = useStore((s) => s.config?.ui.theme ?? DEFAULT_THEME);
  const keymap = useStore((s) => s.keymap);
  const hasSession = useStore((s) => !!s.activeSessionId);
  const inWorkspace = useStore((s) => s.view === 'workspace');

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

  /** The keybound commands, straight from the registry. */
  const boundCommands = (): Command[] =>
    commandRows(keymap, { session: hasSession, workspace: inWorkspace }).map((row) => ({
      ...row,
      action: () => { close(); runCommand(row.id); },
    }));

  /** Palette-only actions: no keybinding, so no registry row. */
  const buildCommands = (): Command[] => {
    // The focused repo's worktree.
    const wt = activeWorktrees.find((w) => w.id === activeWorktreeId)
      ?? activeWorktrees.find((w) => w.repo_id === activeRepoId)
      ?? activeWorktrees[0];
    // Named in the label: the row says which repo it acts on.
    const scope = activeRepos.find((r) => r.id === wt?.repo_id)?.project ?? null;
    const inRepo = (label: string) => (scope ? `${label} — ${scope}` : label);
    const cmds: Command[] = [];
    /** Closes the palette, runs a backend command, reports a failure in the status bar. */
    const send = async (name: string, args: Record<string, unknown>) => {
      close();
      try {
        await invoke(name, args);
      } catch (e) {
        setLastError(e);
      }
    };

    if (activeTask) {
      cmds.push({
        id: 'nav-workspace',
        label: 'Go to Workspace',
        group: 'Navigation',
        icon: PanelsTopLeft,
        action: () => { setView('workspace'); close(); },
      });
      cmds.push({
        id: 'mode-overview',
        label: 'View: Session overview',
        group: 'Workspace',
        icon: Compass,
        action: () => {
          setView('workspace');
          setWorkspaceMode('overview');
          close();
        },
      });
      cmds.push({
        id: 'mode-code',
        label: 'View: Editor & diff',
        group: 'Workspace',
        icon: FileCode,
        action: () => {
          setView('workspace');
          setWorkspaceMode('code');
          close();
        },
      });
      for (const m of DIFF_MODES) {
        cmds.push({
          id: `diff-mode-${m.id}`,
          label: `Diff base: ${m.title}`,
          group: 'Workspace',
          icon: m.Icon,
          action: () => { setDiffMode(m.id); close(); },
        });
      }
      cmds.push({
        id: 'terminal-new',
        label: 'Terminal: New',
        group: 'Workspace',
        icon: Plus,
        action: () => { close(); ensureTerminalTab({ fresh: true }); },
      });

      if (wt) {
        cmds.push({
          id: 'git-push',
          label: inRepo('Git: Push'),
          group: 'Git',
          icon: ArrowUpFromLine,
          action: () => send('push', { worktreeId: wt.id }),
        });
        cmds.push({
          id: 'git-pull',
          label: inRepo('Git: Pull'),
          group: 'Git',
          icon: ArrowDownToLine,
          action: () => send('pull', { worktreeId: wt.id }),
        });
        cmds.push({
          id: 'git-create-mr',
          label: inRepo('Git: Create merge request…'),
          group: 'Git',
          icon: GitPullRequestArrow,
          // Opens the confirmation pre-filled except the text.
          action: () => send('create_mr', { worktreeId: wt.id }),
        });
        cmds.push({
          id: 'git-rebase',
          label: inRepo('Git: Rebase on main'),
          group: 'Git',
          icon: ChevronsUp,
          action: () => send('rebase_on_main', { worktreeId: wt.id }),
        });
      }

      cmds.push({
        id: 'task-pause',
        label: 'Pause task',
        group: 'Task',
        icon: PauseCircle,
        action: () => send('pause_task', { shortId: activeTask.short_id }),
      });
      cmds.push({
        id: 'task-sync',
        label: 'Sync task',
        group: 'Task',
        icon: RefreshCw,
        action: () => send('sync_task', { shortId: activeTask.short_id }),
      });
    }

    for (const t of THEMES) {
      cmds.push({
        id: `theme-${t.id}`,
        label: `Theme: ${t.label}${activeTheme === t.id ? '  ✓' : ''}`,
        group: 'Theme',
        icon: Palette,
        action: () => { setTheme(t.id); close(); },
      });
    }

    return cmds;
  };

  const commands = [...boundCommands(), ...buildCommands()];

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
                  const Icon = cmd.icon;
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
