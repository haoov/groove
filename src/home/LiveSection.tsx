import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { ChevronDown, ChevronRight, Plus, RefreshCw } from 'lucide-react';
import { useStore } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { LiveRepos } from './RepoRow';
import { KIND_LABEL, openTask, priorityRank, rowKey, summarize } from './helpers';
import type { HomeEntry } from '../shared/ipc/ipc';

// Fold state per entry, persisted so Home reopens the way it was left.
const EXPAND_KEY = 'wb.homeExpanded';
function loadExpanded(): Set<string> {
  try { return new Set(JSON.parse(localStorage.getItem(EXPAND_KEY) ?? '[]')); }
  catch { return new Set(); }
}
function saveExpanded(ids: Set<string>) {
  try { localStorage.setItem(EXPAND_KEY, JSON.stringify([...ids])); } catch { /* ignore */ }
}

/** Everything checked out locally: tasks, explorers and reviews with a worktree.
 *  Rendered as the body of the Home "Live" tab — toolbar over the list. */
export function LiveSection({ filter = '', onCount }: { filter?: string; onCount?: (n: number) => void }) {
  const snapshot = useStore((s) => s.homeSnapshot);
  const homeLoading = useStore((s) => s.homeLoading);
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');

  // Attention first, then the busiest working trees; the shared filter narrows.
  const entries = useMemo(() => {
    const score = (e: HomeEntry) => {
      const s = summarize(e);
      return (s.attention ? 1000 : 0) + s.dirty;
    };
    const needle = filter.trim().toLowerCase();
    return [...(snapshot ?? [])]
      .filter((e) => !needle || `${e.short_id} ${e.title} ${e.kind}`.toLowerCase().includes(needle))
      .sort((a, b) => score(b) - score(a) || priorityRank(a.priority) - priorityRank(b.priority));
  }, [snapshot, filter]);

  useEffect(() => { onCount?.(entries.length); }, [entries.length, onCount]);

  const createExplorer = async () => {
    const name = newName.trim();
    setCreating(false);
    setNewName('');
    try {
      await invoke<string>('open_explorer_session', { name: name || null });
    } catch (e) {
      setLastError(String(e));
    }
  };

  return (
    <>
      <div className="home-toolbar">
        {creating ? (
          <span className="explorer-new-composer">
            <input
              className="explorer-new-input"
              autoFocus
              placeholder="Name this explorer…"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') createExplorer();
                if (e.key === 'Escape') { setCreating(false); setNewName(''); }
              }}
            />
            <button className="btn-primary" onClick={createExplorer}>Create</button>
            <button className="btn-secondary" onClick={() => { setCreating(false); setNewName(''); }}>Cancel</button>
          </span>
        ) : (
          <button className="live-btn" onClick={() => { setNewName(''); setCreating(true); }}>
            <Plus size={13} strokeWidth={2} />
            New explorer
          </button>
        )}
        <span className="home-toolbar-spring" />
        <button
          className="home-link"
          title="Refresh (also re-checks CI)"
          onClick={() => refreshHome(true)}
          disabled={homeLoading}
        >
          <RefreshCw size={11} strokeWidth={2} className={homeLoading ? 'spin' : undefined} />
          refresh
        </button>
      </div>

      {snapshot === null ? (
        <p className="home-empty">Loading…</p>
      ) : entries.length === 0 ? (
        <p className="home-empty">
          {filter.trim()
            ? `Nothing checked out matches “${filter.trim()}”.`
            : 'Nothing checked out — open a task from Up next, or start an explorer.'}
        </p>
      ) : (
        <div className="home-rows">
          {entries.map((e) => <LiveRow key={e.short_id} entry={e} />)}
        </div>
      )}
    </>
  );
}

type LiveConfirm = 'finish' | 'delete' | 'discard' | null;

function LiveRow({ entry }: { entry: HomeEntry }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(entry.title);
  // Two-click confirm: the first click arms; the confirm row does the deed.
  const [confirm, setConfirm] = useState<LiveConfirm>(null);
  const [expanded, setExpanded] = useState(() => loadExpanded().has(entry.short_id));

  // Folded, the row states only what it is and how many repos it holds.
  const repoCount = useMemo(
    () => new Set(entry.repos.map((r) => r.repo_id)).size,
    [entry.repos],
  );

  const toggleExpanded = (e: React.MouseEvent) => {
    e.stopPropagation();
    setExpanded((v) => {
      const ids = loadExpanded();
      if (v) ids.delete(entry.short_id); else ids.add(entry.short_id);
      saveExpanded(ids);
      return !v;
    });
  };

  const session = sessionOrder
    .map((id) => sessions[id])
    .find((x) => x?.task?.short_id === entry.short_id);

  const rename = async () => {
    const next = name.trim();
    setRenaming(false);
    if (!next || next === entry.title) return;
    try {
      await invoke('rename_explorer', { shortId: entry.short_id, name: next });
      refreshHome();
    } catch (e) {
      setLastError(String(e));
    }
  };

  // Finish (task → Notion done + teardown), delete (task, no Notion change) and
  // discard (explorer/review) all end in the same place: session gone, Home fresh.
  const runConfirmed = async (action: Exclude<LiveConfirm, null>) => {
    setConfirm(null);
    try {
      if (action === 'discard') {
        if (session) await endSession(session.id);
        await invoke('discard_explorer', { shortId: entry.short_id });
      } else {
        await invoke(action === 'finish' ? 'finish_task' : 'delete_task', { shortId: entry.short_id });
      }
      refreshHome();
    } catch (e) {
      setLastError(String(e));
    }
  };

  return (
    <div className="live-group">
      <div
        className="home-row live-row"
        role="button"
        tabIndex={0}
        onClick={() => !renaming && openTask(entry.short_id)}
        onKeyDown={(e) => {
          if (renaming) return;
          if (e.key === 'Enter') { e.preventDefault(); openTask(entry.short_id); }
        }}
        onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY }); }}
        title="Open the workspace"
      >
        <button
          className="live-caret"
          title={expanded ? 'Fold' : 'Expand'}
          onClick={toggleExpanded}
        >
          {expanded ? <ChevronDown size={12} strokeWidth={2} /> : <ChevronRight size={12} strokeWidth={2} />}
        </button>
        <span className={`type-badge type-${entry.kind}`}>{KIND_LABEL[entry.kind]}</span>
        <span className="row-key">{rowKey(entry)}</span>
        {renaming ? (
          <input
            className="explorer-rename-input"
            autoFocus
            value={name}
            onClick={(e) => e.stopPropagation()}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Enter') rename();
              if (e.key === 'Escape') { setRenaming(false); setName(entry.title); }
            }}
          />
        ) : (
          <span className="row-titleline">
            <span className="row-title">{entry.title}</span>
          </span>
        )}

        <span className="row-summary">
          {repoCount > 0 && (
            <span className="row-note">{repoCount} repo{repoCount === 1 ? '' : 's'}</span>
          )}
        </span>
      </div>

      {expanded && (
        <div className="live-detail">
          <div className="detail-actions">
            <button className="live-btn" onClick={() => openTask(entry.short_id)}>Open workspace</button>
            {entry.kind === 'explorer' && (
              <button className="live-btn" onClick={() => { setName(entry.title); setRenaming(true); }}>Rename</button>
            )}
            {entry.kind === 'task' && (
              <>
                <button className="live-btn go" onClick={() => setConfirm('finish')}>Finish</button>
                <button className="live-btn danger" onClick={() => setConfirm('delete')}>Delete</button>
              </>
            )}
            {entry.kind !== 'task' && (
              <button className="live-btn danger" onClick={() => setConfirm('discard')}>
                {entry.kind === 'review' ? 'Finish review' : 'Discard'}
              </button>
            )}
          </div>
          {entry.repos.length === 0 ? (
            <div className="live-detail-empty">No repos yet — open it to add one.</div>
          ) : (
            <LiveRepos entry={entry} />
          )}
        </div>
      )}

      {confirm && (
        <div className="detail-row confirm">
          <span>
            {confirm === 'finish'
              ? `Finish ${entry.short_id}? Marks it done in Notion and removes its worktrees.`
              : confirm === 'delete'
                ? `Delete ${entry.short_id} locally? Notion is untouched; the worktrees are removed.`
                : `Discard ${entry.short_id} and delete its worktrees?`}
          </span>
          <button className="live-btn danger" onClick={() => runConfirmed(confirm)}>
            {confirm === 'finish' ? 'Yes, finish' : confirm === 'delete' ? 'Yes, delete' : 'Yes, discard'}
          </button>
          <button className="live-btn" onClick={() => setConfirm(null)}>Cancel</button>
        </div>
      )}

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} className="context-menu">
          <button className="context-item" onClick={() => { setMenu(null); openTask(entry.short_id); }}>
            Open workspace
          </button>
          {entry.kind === 'explorer' && (
            <button className="context-item" onClick={() => { setMenu(null); setName(entry.title); setRenaming(true); }}>
              Rename
            </button>
          )}
          {entry.kind === 'task' && (
            <>
              <button className="context-item" onClick={() => { setMenu(null); setConfirm('finish'); }}>
                Finish task
              </button>
              <button className="context-item" onClick={() => { setMenu(null); setConfirm('delete'); }}>
                Delete locally
              </button>
            </>
          )}
          {entry.kind !== 'task' && (
            <button className="context-item" onClick={() => { setMenu(null); setConfirm('discard'); }}>
              {entry.kind === 'review' ? 'Finish review' : 'Discard'}
            </button>
          )}
        </ContextMenu>
      )}
    </div>
  );
}
