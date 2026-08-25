import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { openTask, priorityLabel, priorityRank } from './helpers';
import { appliesTo, matchesQuery, parseQuery, type CountReport } from './filter';
import { statusKey, STATUS_RANK } from '../shared/lib/taskStatus';
import type { Task } from '../shared/ipc/ipc';

// Up next = the queued tasks not yet checked out. Columns: id · name ·
// priority · status. Reviews live in their own tab now.

/** The fields an Up next row can answer — see `appliesTo`. */
const FIELDS = ['id', 'title', 'status', 'priority', 'provider'];

const HIDDEN_KEY = 'wb.homeHiddenTasks';
function loadHidden(): Set<string> {
  try { return new Set(JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? '[]')); }
  catch { return new Set(); }
}
function saveHidden(keys: Set<string>) {
  try { localStorage.setItem(HIDDEN_KEY, JSON.stringify([...keys])); } catch { /* ignore */ }
}

export function UpNextSection({ filter = '', onCount }: { filter?: string; onCount?: CountReport }) {
  const tasks = useStore((s) => s.tasks);
  const refreshTasks = useStore((s) => s.refreshTasks);
  const snapshot = useStore((s) => s.homeSnapshot);
  const setLastError = useStore((s) => s.setLastError);
  const [menu, setMenu] = useState<{ x: number; y: number; task: Task } | null>(null);
  const [hidden, setHidden] = useState<Set<string>>(loadHidden);
  const [showHidden, setShowHidden] = useState(false);

  const toggleHidden = (key: string) => {
    setHidden((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      saveHidden(next);
      return next;
    });
  };

  useEffect(() => {
    if (tasks.length === 0) refreshTasks();
  }, [tasks.length, refreshTasks]);

  const { items, hiddenCount } = useMemo(() => {
    const live = new Set((snapshot ?? []).map((e) => e.short_id));
    const q = parseQuery(filter);
    const all = tasks
      .filter((t) => !live.has(t.short_id) && statusKey(t.status) !== 'done')
      .filter((t) => matchesQuery(q, `${t.short_id} ${t.title} ${t.status} ${t.priority ?? ''}`, {
        id: t.short_id,
        title: t.title,
        status: t.status,
        priority: t.priority,
        provider: t.provider,
      }))
      .sort((a, b) => {
        const s = (STATUS_RANK[statusKey(a.status)] ?? 9) - (STATUS_RANK[statusKey(b.status)] ?? 9);
        return s !== 0 ? s : priorityRank(a.priority) - priorityRank(b.priority);
      });
    const hiddenN = all.filter((t) => hidden.has(t.short_id)).length;
    return { items: showHidden ? all : all.filter((t) => !hidden.has(t.short_id)), hiddenCount: hiddenN };
  }, [tasks, snapshot, filter, hidden, showHidden]);

  const applicable = useMemo(() => appliesTo(parseQuery(filter), FIELDS), [filter]);
  useEffect(() => { onCount?.(items.length, applicable, filter); }, [items.length, applicable, filter, onCount]);

  return (
    <div className="upnext-root" onClick={() => setMenu(null)}>
      {(hiddenCount > 0 || showHidden) && (
        <div className="home-toolbar">
          <button
            className={`home-link${showHidden ? ' active' : ''}`}
            onClick={() => setShowHidden((v) => !v)}
            title="Rows you hid with right-click — reveal them to unhide"
          >
            {showHidden ? 'done' : `hidden (${hiddenCount})`}
          </button>
        </div>
      )}

      {items.length === 0 ? (
        <p className="home-empty">
          {filter.trim() ? `No task matches “${filter.trim()}”.` : 'Nothing queued — every task is checked out or done.'}
        </p>
      ) : (
        <div className="upnext-table tasks-table">
          <div className="upnext-head">
            <span>name</span>
            <span>priority</span>
            <span>status</span>
            <span>provider</span>
          </div>
          {items.map((task) => (
            <button
              key={task.short_id}
              className={`upnext-tr${hidden.has(task.short_id) ? ' dimmed' : ''}`}
              onClick={() => openTask(task.short_id)}
              onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY, task }); }}
              title={task.title}
            >
              <span className="row-titleline"><span className="row-title">{task.title}</span></span>
              <span className="upnext-prio">
                {task.priority
                  ? <span className={`prio-badge p${priorityRank(task.priority)}`}>{priorityLabel(task.priority)}</span>
                  : <span className="row-note">—</span>}
              </span>
              <span className="upnext-status">
                <span className={`row-status status-${statusKey(task.status)}`}>{task.status}</span>
              </span>
              <span className="row-provider">{task.provider ?? '—'}</span>
            </button>
          ))}
        </div>
      )}

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} className="context-menu">
          <button className="context-item" onClick={() => { openTask(menu.task.short_id); setMenu(null); }}>
            Open workspace
          </button>
          <button
            className="context-item"
            onClick={async () => {
              try { await invoke('sync_task', { shortId: menu.task.short_id }); }
              catch (e) { setLastError(String(e)); }
              setMenu(null);
            }}
          >
            Sync
          </button>
          <button className="context-item" onClick={() => { toggleHidden(menu.task.short_id); setMenu(null); }}>
            {hidden.has(menu.task.short_id) ? 'Unhide' : 'Hide from Up next'}
          </button>
        </ContextMenu>
      )}
    </div>
  );
}
