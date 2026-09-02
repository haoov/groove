import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { ChevronDown, ChevronRight, PanelsTopLeft, Pencil, Check, Trash2 } from 'lucide-react';
import { useStore } from '../shared/store';
import { endSession } from '../shared/lib/endSession';
import { openExternal } from '../shared/lib/openExternal';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { LiveRepos } from './RepoRow';
import { KIND_LABEL, openTask, priorityRank, rowProvider, summarize } from './helpers';
import { appliesTo, matchesQuery, parseQuery, type CountReport } from './filter';
import { providerCopy } from '../shared/lib/taskProvider';

import type { HomeEntry } from '../shared/ipc/ipc';

/** The fields a Live row can answer — see `appliesTo`. */
const FIELDS = ['id', 'title', 'kind', 'status', 'priority', 'provider', 'forge', 'repo', 'branch', 'mr'];

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
export function LiveSection({ filter = '', onCount }: { filter?: string; onCount?: CountReport }) {
  const snapshot = useStore((s) => s.homeSnapshot);

  // Attention first, then the busiest working trees; the shared filter narrows.
  const entries = useMemo(() => {
    const score = (e: HomeEntry) => {
      const s = summarize(e);
      return (s.attention ? 1000 : 0) + s.dirty;
    };
    const q = parseQuery(filter);
    return [...(snapshot ?? [])]
      .filter((e) => matchesQuery(q, `${e.short_id} ${e.title} ${e.kind}`, {
        id: e.short_id,
        title: e.title,
        kind: e.kind,
        status: e.status,
        priority: e.priority,
        provider: e.provider,
        repo: e.repos.map((r) => r.project),
        branch: e.repos.map((r) => r.branch ?? ''),
        mr: e.repos.map((r) => r.mr?.remote_id ?? ''),
        // Where the code is hosted, not where the task came from.
        forge: e.repos.map((r) => r.mr?.platform ?? ''),
      }))
      .sort((a, b) => score(b) - score(a) || priorityRank(a.priority) - priorityRank(b.priority));
  }, [snapshot, filter]);

  const applicable = useMemo(() => appliesTo(parseQuery(filter), FIELDS), [filter]);
  // Report the filter the count belongs to: Home must not route on a stale count.
  useEffect(() => { onCount?.(entries.length, applicable, filter); }, [entries.length, applicable, filter, onCount]);

  return (
    <>
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
  // Copy that names this task's own source, so a confirm says "in Notion"
  // rather than "at its source".
  const src = providerCopy(entry);

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

  // Finish (task → done at its source + teardown), delete (local only) and
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
        {entry.external_url ? (
          <button
            className={`type-badge type-${entry.kind} type-badge--link`}
            title={`Open in ${src.label}`}
            onClick={(e) => { e.stopPropagation(); openExternal(entry.external_url!); }}
          >
            {KIND_LABEL[entry.kind]}
          </button>
        ) : (
          <span className={`type-badge type-${entry.kind}`}>{KIND_LABEL[entry.kind]}</span>
        )}
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
        <span className="row-provider">{rowProvider(entry)}</span>

        <span className="row-summary">
          {repoCount > 0 && (
            <span className="row-note">{repoCount} repo{repoCount === 1 ? '' : 's'}</span>
          )}
        </span>
      </div>

      {expanded && (
        <div className="live-detail">
          <div className="detail-actions">
            <button className="live-btn live-btn--icon" title="Open workspace" onClick={() => openTask(entry.short_id)}>
              <PanelsTopLeft size={15} strokeWidth={1.75} />
            </button>
            {entry.kind === 'explorer' && (
              <button className="live-btn live-btn--icon" title="Rename" onClick={() => { setName(entry.title); setRenaming(true); }}>
                <Pencil size={15} strokeWidth={1.75} />
              </button>
            )}
            {entry.kind === 'task' && (
              <>
                <button className="live-btn live-btn--icon go" title="Finish" onClick={() => setConfirm('finish')}>
                  <Check size={16} strokeWidth={2} />
                </button>
                <button className="live-btn live-btn--icon danger" title="Delete" onClick={() => setConfirm('delete')}>
                  <Trash2 size={15} strokeWidth={1.75} />
                </button>
              </>
            )}
            {entry.kind !== 'task' && (
              <button
                className="live-btn live-btn--icon danger"
                title={entry.kind === 'review' ? 'Finish review' : 'Discard'}
                onClick={() => setConfirm('discard')}
              >
                {entry.kind === 'review' ? <Check size={16} strokeWidth={2} /> : <Trash2 size={15} strokeWidth={1.75} />}
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
          {/* Name what happens AT THE SOURCE: this is the last step before an
              action that reaches outside the app. */}
          <span>
            {confirm === 'finish'
              ? `Finish ${entry.short_id}? ${src.finish} Its worktrees are removed.`
              : confirm === 'delete'
                ? `Delete ${entry.short_id}? ${src.discard} Its worktrees are removed.`
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
          {entry.external_url && (
            <button
              className="context-item"
              title={entry.external_url}
              onClick={() => { setMenu(null); openExternal(entry.external_url!); }}
            >
              Open in {src.label}
            </button>
          )}
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
                Delete task
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
