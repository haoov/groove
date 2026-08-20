import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, RefreshCw } from 'lucide-react';
import { useStore } from '../shared/store';
import { endSession } from '../app/providers/useIpc';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { RepoRow } from './RepoRow';
import { KIND_LABEL, openTask, priorityLabel, priorityRank, rowKey, summarize } from './helpers';
import type { HomeEntry } from '../shared/ipc/ipc';

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

function LiveRow({ entry }: { entry: HomeEntry }) {
  const sessions = useStore((s) => s.sessions);
  const sessionOrder = useStore((s) => s.sessionOrder);
  const agentActivity = useStore((s) => s.agentActivity);
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(entry.title);
  const [confirmDiscard, setConfirmDiscard] = useState(false);

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

  const discard = async () => {
    setConfirmDiscard(false);
    try {
      if (session) await endSession(session.id);
      await invoke('discard_explorer', { shortId: entry.short_id });
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

        {/* Nothing aggregated here: each repo reports its own git status and MR
            below, so the only thing left is the agent — and whether it's stuck
            waiting on an answer in a terminal you may not be looking at. */}
        <span className="row-summary">
          {agentLive && (
            <span className={agentWaiting ? 'stat-agent waiting' : 'stat-agent'}>
              {agentWaiting ? 'agent · waiting' : 'agent'}
            </span>
          )}
        </span>
      </div>

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
          {entry.kind !== 'task' && (
            <button className="home-link danger" onClick={() => setConfirmDiscard(true)}>
              {entry.kind === 'review' ? 'finish review' : 'discard'}
            </button>
          )}
        </div>
      </div>

      {confirmDiscard && (
        <div className="detail-row confirm">
          <span>Discard {entry.short_id} and delete its worktrees?</span>
          <button className="home-link danger" onClick={discard}>yes, discard</button>
          <button className="home-link" onClick={() => setConfirmDiscard(false)}>cancel</button>
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
          {entry.kind !== 'task' && (
            <button className="context-item" onClick={() => { setMenu(null); setConfirmDiscard(true); }}>
              {entry.kind === 'review' ? 'Finish review' : 'Discard'}
            </button>
          )}
        </ContextMenu>
      )}
    </div>
  );
}
