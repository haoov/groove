import { useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { ChevronDown, ChevronRight, Plus, RefreshCw } from 'lucide-react';
import { useStore } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { RepoRow } from './RepoRow';
import { KIND_LABEL, openTask, priorityLabel, priorityRank, rowKey, summarize } from './helpers';
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

/** Everything checked out locally: tasks, explorers and reviews with a worktree. */
export function LiveSection() {
  const snapshot = useStore((s) => s.homeSnapshot);
  const homeLoading = useStore((s) => s.homeLoading);
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');

  // Attention first, then the busiest working trees.
  const entries = useMemo(() => {
    const score = (e: HomeEntry) => {
      const s = summarize(e);
      return (s.attention ? 1000 : 0) + s.dirty;
    };
    return [...(snapshot ?? [])].sort(
      (a, b) => score(b) - score(a) || priorityRank(a.priority) - priorityRank(b.priority),
    );
  }, [snapshot]);

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
    <section className="home-section">
      <h2 className="home-heading">
        Live
        {entries.length > 0 && <span className="home-heading-count">{entries.length}</span>}
        <span className="home-heading-actions">
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
            <button className="home-link" onClick={() => { setNewName(''); setCreating(true); }}>
              <Plus size={11} strokeWidth={2.2} />
              new explorer
            </button>
          )}
          <button
            className="home-link"
            title="Refresh (also re-checks CI)"
            onClick={() => refreshHome(true)}
            disabled={homeLoading}
          >
            <RefreshCw size={11} strokeWidth={2} className={homeLoading ? 'spin' : undefined} />
            refresh
          </button>
        </span>
      </h2>

      {snapshot === null ? (
        <p className="home-empty">Loading…</p>
      ) : entries.length === 0 ? (
        <p className="home-empty">
          Nothing checked out — open a task from the queue below, or start an explorer.
        </p>
      ) : (
        <div className="home-rows">
          {entries.map((e) => <LiveRow key={e.short_id} entry={e} />)}
        </div>
      )}
    </section>
  );
}

type LiveConfirm = 'finish' | 'delete' | 'discard' | null;

function LiveRow({ entry }: { entry: HomeEntry }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const agentActivity = useStore((s) => s.agentActivity);
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(entry.title);
  // Two-click confirm: the first click arms; the confirm row does the deed.
  const [confirm, setConfirm] = useState<LiveConfirm>(null);
  const [expanded, setExpanded] = useState(() => loadExpanded().has(entry.short_id));
  const summary = summarize(entry);

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
  const agentLive = !!session?.ptySessions.some((p) => p.ptyType === 'agent');
  const agentWaiting = agentActivity[entry.short_id]?.state === 'waiting';

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
            {entry.priority && (
              <span className={`prio-badge p${priorityRank(entry.priority)}`}>
                {priorityLabel(entry.priority)}
              </span>
            )}
          </span>
        )}

        {/* Folded: the aggregates stand in for the hidden repo rows. Expanded:
            each repo reports its own state below, so only the agent chip stays. */}
        <span className="row-summary">
          {!expanded && (
            <>
              {(summary.added > 0 || summary.deleted > 0) && (
                <span className="row-chip">
                  {summary.added > 0 && <span className="stat-add">+{summary.added}</span>}
                  {summary.deleted > 0 && <span className="stat-del">−{summary.deleted}</span>}
                </span>
              )}
              {summary.ahead > 0 && <span className="row-chip stat-ahead">↑{summary.ahead}</span>}
              {summary.behind > 0 && <span className="row-chip stat-behind">↓{summary.behind}</span>}
              {summary.mrs > 0 && (
                <span className={`row-chip${summary.ciFail ? ' stat-bad' : ''}`}>
                  {summary.mrs === 1 ? 'MR' : `${summary.mrs} MRs`}
                  {summary.ciFail && ' · ci failed'}
                  {summary.unresolved > 0 && ` · ${summary.unresolved} open`}
                </span>
              )}
            </>
          )}
          {agentLive && (
            <span className={agentWaiting ? 'stat-agent waiting' : 'stat-agent'}>
              {agentWaiting ? 'agent · waiting' : 'agent'}
            </span>
          )}
        </span>
      </div>

      {expanded && (
        <div className="live-detail">
          {entry.repos.length === 0 ? (
            <div className="detail-row muted">No repos yet — open it to add one.</div>
          ) : (
            entry.repos.map((repo) => <RepoRow key={repo.repo_id} entry={entry} repo={repo} />)
          )}
          <div className="detail-actions">
            <button className="home-link" onClick={() => openTask(entry.short_id)}>open workspace</button>
            {entry.kind === 'explorer' && (
              <button className="home-link" onClick={() => { setName(entry.title); setRenaming(true); }}>rename</button>
            )}
            {entry.kind === 'task' && (
              <>
                <button className="home-link" onClick={() => setConfirm('finish')}>finish</button>
                <button className="home-link danger" onClick={() => setConfirm('delete')}>delete</button>
              </>
            )}
            {entry.kind !== 'task' && (
              <button className="home-link danger" onClick={() => setConfirm('discard')}>
                {entry.kind === 'review' ? 'finish review' : 'discard'}
              </button>
            )}
          </div>
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
          <button className="home-link danger" onClick={() => runConfirmed(confirm)}>
            {confirm === 'finish' ? 'yes, finish' : confirm === 'delete' ? 'yes, delete' : 'yes, discard'}
          </button>
          <button className="home-link" onClick={() => setConfirm(null)}>cancel</button>
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
