import { useCallback, useEffect, useMemo, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { openTask, priorityLabel, priorityRank } from './helpers';
import { statusKey, STATUS_RANK } from '../shared/lib/taskStatus';
import type { Task } from '../shared/ipc/ipc';

// Up next = the queued Notion tasks not yet checked out. Columns: id · name ·
// priority · status. Reviews live in their own tab now.

const HIDDEN_KEY = 'wb.homeHiddenTasks';
function loadHidden(): Set<string> {
  try { return new Set(JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? '[]')); }
  catch { return new Set(); }
}
function saveHidden(keys: Set<string>) {
  try { localStorage.setItem(HIDDEN_KEY, JSON.stringify([...keys])); } catch { /* ignore */ }
}

export function UpNextSection({ filter = '', onCount }: { filter?: string; onCount?: (n: number) => void }) {
  const tasks = useStore((s) => s.tasks);
  const setTasks = useStore((s) => s.setTasks);
  const snapshot = useStore((s) => s.homeSnapshot);
  const setSyncStatus = useStore((s) => s.setSyncStatus);
  const setLastError = useStore((s) => s.setLastError);
  const [menu, setMenu] = useState<{ x: number; y: number; task: Task } | null>(null);
  const [refreshing, setRefreshing] = useState(false);
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

  const loadTasks = useCallback(async () => {
    setSyncStatus('syncing');
    try {
      setTasks(await invoke<Task[]>('list_tasks'));
      setSyncStatus('idle');
    } catch (e) {
      setSyncStatus('error');
      setLastError(String(e));
    }
  }, [setSyncStatus, setTasks, setLastError]);

  useEffect(() => {
    if (tasks.length === 0) loadTasks();
  }, [tasks.length, loadTasks]);

  const { items, hiddenCount } = useMemo(() => {
    const live = new Set((snapshot ?? []).map((e) => e.short_id));
    const needle = filter.trim().toLowerCase();
    const all = tasks
      .filter((t) => !live.has(t.short_id) && statusKey(t.status) !== 'done')
      .filter((t) => !needle || `${t.short_id} ${t.title} ${t.status} ${t.priority ?? ''}`.toLowerCase().includes(needle))
      .sort((a, b) => {
        const s = (STATUS_RANK[statusKey(a.status)] ?? 9) - (STATUS_RANK[statusKey(b.status)] ?? 9);
        return s !== 0 ? s : priorityRank(a.priority) - priorityRank(b.priority);
      });
    const hiddenN = all.filter((t) => hidden.has(t.short_id)).length;
    return { items: showHidden ? all : all.filter((t) => !hidden.has(t.short_id)), hiddenCount: hiddenN };
  }, [tasks, snapshot, filter, hidden, showHidden]);

  useEffect(() => { onCount?.(items.length); }, [items.length, onCount]);

  return (
    <div className="upnext-root" onClick={() => setMenu(null)}>
      <div className="home-toolbar">
        {(hiddenCount > 0 || showHidden) && (
          <button
            className={`home-link${showHidden ? ' active' : ''}`}
            onClick={() => setShowHidden((v) => !v)}
            title="Rows you hid with right-click — reveal them to unhide"
          >
            {showHidden ? 'done' : `hidden (${hiddenCount})`}
          </button>
        )}
        <span className="home-toolbar-spring" />
        <button
          className="home-link"
          onClick={async () => {
            if (refreshing) return;
            setRefreshing(true);
            try { await loadTasks(); } finally { setRefreshing(false); }
          }}
          title="Refresh tasks from Notion"
        >
          <RefreshCw size={11} strokeWidth={2.2} className={refreshing ? 'spin' : undefined} />
          refresh
        </button>
      </div>

      {items.length === 0 ? (
        <p className="home-empty">
          {filter.trim() ? `No task matches “${filter.trim()}”.` : 'Nothing queued — every task is checked out or done.'}
        </p>
      ) : (
        <div className="upnext-table tasks-table">
          <div className="upnext-head">
            <span>code</span>
            <span>name</span>
            <span>priority</span>
            <span>status</span>
          </div>
          {items.map((task) => (
            <button
              key={task.short_id}
              className={`upnext-tr${hidden.has(task.short_id) ? ' dimmed' : ''}`}
              onClick={() => openTask(task.short_id)}
              onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY, task }); }}
              title={task.title}
            >
              <span className="row-key">{task.short_id}</span>
              <span className="row-titleline"><span className="row-title">{task.title}</span></span>
              <span className="upnext-prio">
                {task.priority
                  ? <span className={`prio-badge p${priorityRank(task.priority)}`}>{priorityLabel(task.priority)}</span>
                  : <span className="row-note">—</span>}
              </span>
              <span className="upnext-status">
                <span className={`row-status status-${statusKey(task.status)}`}>{task.status}</span>
              </span>
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
            Sync from Notion
          </button>
          <button className="context-item" onClick={() => { toggleHidden(menu.task.short_id); setMenu(null); }}>
            {hidden.has(menu.task.short_id) ? 'Unhide' : 'Hide from Up next'}
          </button>
        </ContextMenu>
      )}
    </div>
  );
}
