import { useCallback, useEffect, useMemo, useState } from 'react';
import { RefreshCw } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { openTask, priorityRank } from './helpers';
import { statusKey, STATUS_RANK } from '../shared/lib/taskStatus';
import type { MainRepo, ReviewMr, Task } from '../shared/ipc/ipc';

/** Rough "how long ago", for the MR age column. */
function timeAgo(iso: string): string {
  const then = new Date(iso).getTime();
  if (!Number.isFinite(then)) return '';
  const s = Math.max(0, (Date.now() - then) / 1000);
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))}m`;
  if (s < 86_400) return `${Math.round(s / 3600)}h`;
  if (s < 86_400 * 7) return `${Math.round(s / 86_400)}d`;
  return `${Math.round(s / 86_400 / 7)}w`;
}

// Review requests and queued tasks answer the same question — "what do I pick up
// next?" — so they share ONE table: code · name · state · owner. Reviews are
// pinned above tasks rather than interleaved: there are only ever a few, and
// someone else is blocked on them.

const UPNEXT_PREVIEW = 10;

// Rows you dismissed with right-click stay hidden across reloads until you reveal
// them — they are noise you already triaged, not gone for good.
const HIDDEN_KEY = 'wb.homeHidden';
function loadHidden(): Set<string> {
  try { return new Set(JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? '[]')); }
  catch { return new Set(); }
}
function saveHidden(keys: Set<string>) {
  try { localStorage.setItem(HIDDEN_KEY, JSON.stringify([...keys])); } catch { /* ignore */ }
}

type UpNextItem =
  | { kind: 'review'; key: string; mr: ReviewMr }
  | { kind: 'task'; key: string; task: Task };

/** The filter matches what the table shows: code, name, state, owner. */
function itemText(item: UpNextItem): string {
  if (item.kind === 'review') {
    const { mr } = item;
    return `${sigil(mr)}${mr.iid} ${mr.title} ${mr.project_full} ${mr.author}`.toLowerCase();
  }
  const { task } = item;
  return `${task.short_id} ${task.title} ${task.status} ${task.priority ?? ''}`.toLowerCase();
}

export function UpNextSection() {
  const tasks = useStore((s) => s.tasks);
  const setTasks = useStore((s) => s.setTasks);
  const snapshot = useStore((s) => s.homeSnapshot);
  const reviewQueue = useStore((s) => s.reviewQueue);
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  const setSyncStatus = useStore((s) => s.setSyncStatus);
  const setLastError = useStore((s) => s.setLastError);
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; item: UpNextItem } | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [filter, setFilter] = useState('');
  // Approved reviews are soft-hidden: someone signed off, so they are no longer
  // waiting on YOU — but they stay one toggle away until they merge.
  const [showApproved, setShowApproved] = useState(false);
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

  // Anything already open as a review session belongs to Live, not here.
  const startedReviews = useMemo(() => {
    const keys = new Set<string>();
    for (const e of snapshot ?? []) {
      if (e.kind !== 'review') continue;
      for (const r of e.repos) if (r.mr) keys.add(`${r.project}!${r.mr.remote_id}`);
    }
    return keys;
  }, [snapshot]);

  const { items, approvedHidden, hiddenCount } = useMemo(() => {
    const pending = (reviewQueue ?? []).filter(
      (mr) => !startedReviews.has(`${mr.project_full.split('/').pop()}!${mr.iid}`),
    );
    const approvedCount = pending.filter((mr) => mr.approved).length;
    const reviews: UpNextItem[] = pending
      .filter((mr) => showApproved || !mr.approved)
      .map((mr) => ({ kind: 'review' as const, key: `${mr.project_full}!${mr.iid}`, mr }));

    const live = new Set((snapshot ?? []).map((e) => e.short_id));
    const queued: UpNextItem[] = tasks
      .filter((t) => !live.has(t.short_id) && statusKey(t.status) !== 'done')
      .sort((a, b) => {
        const s = (STATUS_RANK[statusKey(a.status)] ?? 9) - (STATUS_RANK[statusKey(b.status)] ?? 9);
        return s !== 0 ? s : priorityRank(a.priority) - priorityRank(b.priority);
      })
      .map((t) => ({ kind: 'task' as const, key: t.short_id, task: t }));

    let all = [...reviews, ...queued];
    const needle = filter.trim().toLowerCase();
    if (needle) all = all.filter((i) => itemText(i).includes(needle));
    const hiddenN = all.filter((i) => hidden.has(i.key)).length;
    const visible = showHidden ? all : all.filter((i) => !hidden.has(i.key));
    return {
      items: visible,
      approvedHidden: showApproved ? 0 : approvedCount,
      hiddenCount: hiddenN,
    };
  }, [reviewQueue, startedReviews, snapshot, tasks, filter, showApproved, hidden, showHidden]);

  const openReview = async (mr: ReviewMr) => {
    const key = `${mr.project_full}!${mr.iid}`;
    if (busy) return;
    setBusy(key);
    try {
      let localPath = mr.local_path;
      if (!localPath) {
        const host = new URL(mr.web_url).host;
        const repo = await invoke<MainRepo>('clone_repo', { url: `git@${host}:${mr.project_full}.git` });
        localPath = repo.local_path;
        refreshReviewQueue();
      }
      await invoke('open_review_session', {
        projectFull: mr.project_full,
        iid: mr.iid,
        title: mr.title,
        sourceBranch: mr.source_branch,
        targetBranch: mr.target_branch,
        webUrl: mr.web_url,
        localPath,
      });
    } catch (e) {
      setLastError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const shown = expanded || filter.trim() ? items : items.slice(0, UPNEXT_PREVIEW);

  return (
    <section className="home-section" onClick={() => setMenu(null)}>
      <h2 className="home-heading">
        Up next
        {items.length > 0 && <span className="home-heading-count">{items.length}</span>}
        <span className="home-heading-actions">
          <input
            className="upnext-filter"
            placeholder="Filter…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            onKeyDown={(e) => { if (e.key === 'Escape') setFilter(''); }}
          />
          {(approvedHidden > 0 || showApproved) && (
            <button
              className={`home-link${showApproved ? ' active' : ''}`}
              onClick={() => setShowApproved((v) => !v)}
              title="Approved reviews are no longer waiting on you — shown on demand"
            >
              {showApproved ? 'hide approved' : `approved (${approvedHidden})`}
            </button>
          )}
          {(hiddenCount > 0 || showHidden) && (
            <button
              className={`home-link${showHidden ? ' active' : ''}`}
              onClick={() => setShowHidden((v) => !v)}
              title="Rows you hid with right-click — reveal them to unhide"
            >
              {showHidden ? 'done' : `hidden (${hiddenCount})`}
            </button>
          )}
          {/* The review queue is polled every ~5 min, which is too slow when you
              know someone just asked. Refreshes the tasks too — both feed this list. */}
          <button
            className="home-link"
            onClick={async () => {
              if (refreshing) return;
              setRefreshing(true);
              try {
                await Promise.all([refreshReviewQueue(), loadTasks()]);
              } finally {
                setRefreshing(false);
              }
            }}
            title="Refresh review requests and tasks"
          >
            <RefreshCw size={11} strokeWidth={2.2} className={refreshing ? 'spin' : undefined} />
            refresh
          </button>
        </span>
      </h2>

      {items.length === 0 ? (
        <p className="home-empty">
          {filter.trim()
            ? `Nothing matches “${filter.trim()}”.`
            : 'Nothing waiting — no review requests, no queued tasks.'}
        </p>
      ) : (
        <div className="upnext-table">
          <div className="upnext-head">
            <span />
            <span>code</span>
            <span>name</span>
            <span>state</span>
            <span>owner</span>
          </div>
          {shown.map((item) =>
            item.kind === 'review' ? (
              <ReviewRow
                key={item.key}
                mr={item.mr}
                busy={busy === item.key}
                dimmed={hidden.has(item.key)}
                onOpen={() => openReview(item.mr)}
                onMenu={(x, y) => setMenu({ x, y, item })}
              />
            ) : (
              <TaskRow
                key={item.key}
                task={item.task}
                dimmed={hidden.has(item.key)}
                onMenu={(x, y) => setMenu({ x, y, item })}
              />
            ),
          )}
          {!filter.trim() && items.length > UPNEXT_PREVIEW && (
            <button className="home-link upnext-more" onClick={() => setExpanded((v) => !v)}>
              {expanded ? 'show less' : `show ${items.length - UPNEXT_PREVIEW} more`}
            </button>
          )}
        </div>
      )}

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} className="context-menu">
          {menu.item.kind === 'task' ? (
            <>
              <button className="context-item" onClick={() => { openTask(menu.item.kind === 'task' ? menu.item.task.short_id : ''); setMenu(null); }}>
                Open workspace
              </button>
              <button
                className="context-item"
                onClick={async () => {
                  const task = menu.item.kind === 'task' ? menu.item.task : null;
                  if (task) {
                    try {
                      await invoke('sync_task', { shortId: task.short_id });
                    } catch (e) {
                      setLastError(String(e));
                    }
                  }
                  setMenu(null);
                }}
              >
                Sync from Notion
              </button>
            </>
          ) : (
            <button className="context-item" onClick={() => { if (menu.item.kind === 'review') openReview(menu.item.mr); setMenu(null); }}>
              Open review
            </button>
          )}
          <button className="context-item" onClick={() => { toggleHidden(menu.item.key); setMenu(null); }}>
            {hidden.has(menu.item.key) ? 'Unhide' : 'Hide from Up next'}
          </button>
        </ContextMenu>
      )}
    </section>
  );
}

/** GitHub numbers PRs with `#`, GitLab with `!`. */
const sigil = (mr: ReviewMr) => (mr.platform === 'github' ? '#' : '!');

/** An MR waiting on the user: code = the reference, state = the project. */
function ReviewRow({
  mr, busy, dimmed, onOpen, onMenu,
}: {
  mr: ReviewMr;
  busy: boolean;
  dimmed: boolean;
  onOpen: () => void;
  onMenu: (x: number, y: number) => void;
}) {
  return (
    <button
      className={`upnext-tr is-review${dimmed ? ' dimmed' : ''}`}
      onClick={onOpen}
      onContextMenu={(e) => { e.preventDefault(); onMenu(e.clientX, e.clientY); }}
      disabled={busy}
      title={`${mr.title}\n${mr.project_full}${sigil(mr)}${mr.iid}\n${mr.source_branch} → ${mr.target_branch}`}
    >
      <span className="type-badge type-review">review</span>
      <span className="row-key">{sigil(mr)}{mr.iid}</span>
      <span className="row-titleline">
        <span className="row-title">{mr.title}</span>
        {mr.draft && <span className="row-note">draft</span>}
        {mr.approved && (
          <span className="approved-badge" title="You/someone approved this — not merged yet">
            approved
          </span>
        )}
      </span>
      <span className="row-note upnext-state" title={`Updated ${timeAgo(mr.updated_at)} ago · ${mr.project_full}`}>
        asked {timeAgo(mr.updated_at)} ago
      </span>
      <span className="row-note upnext-owner">{busy ? 'opening…' : mr.author}</span>
    </button>
  );
}

/** A queued task: state = its Notion status. */
function TaskRow({
  task, dimmed, onMenu,
}: {
  task: Task;
  dimmed: boolean;
  onMenu: (x: number, y: number) => void;
}) {
  return (
    <button
      className={`upnext-tr${dimmed ? ' dimmed' : ''}`}
      onClick={() => openTask(task.short_id)}
      onContextMenu={(e) => { e.preventDefault(); onMenu(e.clientX, e.clientY); }}
      title={task.title}
    >
      <span className="type-badge type-task">task</span>
      <span className="row-key">{task.short_id}</span>
      <span className="row-titleline">
        <span className="row-title">{task.title}</span>
      </span>
      <span className="upnext-state">
        <span className={`row-status status-${statusKey(task.status)}`}>{task.status}</span>
      </span>
      <span className="row-note upnext-owner">—</span>
    </button>
  );
}
