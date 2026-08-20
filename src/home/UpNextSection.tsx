import { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, RefreshCw } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { ContextMenu } from '../shared/ui/ContextMenu';
import { NewTaskModal } from './NewTaskModal';
import { openTask, priorityLabel, priorityRank } from './helpers';
import { statusKey, STATUS_RANK } from '../shared/lib/taskStatus';
import type { MainRepo, ReviewMr, Task } from '../shared/ipc/ipc';

// Review requests and queued tasks answer the same question — "what do I pick up
// next?" — so they share ONE table: code · name · state · owner. Reviews are
// pinned above tasks rather than interleaved: there are only ever a few, and
// someone else is blocked on them.

const UPNEXT_PREVIEW = 10;

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
  const [menu, setMenu] = useState<{ x: number; y: number; task: Task } | null>(null);
  const [composing, setComposing] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [filter, setFilter] = useState('');
  // Approved reviews are soft-hidden: someone signed off, so they are no longer
  // waiting on YOU — but they stay one toggle away until they merge.
  const [showApproved, setShowApproved] = useState(false);

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

  const { items, approvedHidden } = useMemo(() => {
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
    return { items: all, approvedHidden: showApproved ? 0 : approvedCount };
  }, [reviewQueue, startedReviews, snapshot, tasks, filter, showApproved]);

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
          <button className="home-link" onClick={() => setComposing((v) => !v)}>
            <Plus size={11} strokeWidth={2.2} />
            new task
          </button>
        </span>
      </h2>

      {composing && <NewTaskModal onClose={() => setComposing(false)} />}

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
                onOpen={() => openReview(item.mr)}
              />
            ) : (
              <TaskRow
                key={item.key}
                task={item.task}
                onMenu={(x, y) => setMenu({ x, y, task: item.task })}
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
          <button className="context-item" onClick={() => { openTask(menu.task.short_id); setMenu(null); }}>
            Open workspace
          </button>
          <button
            className="context-item"
            onClick={async () => {
              try {
                await invoke('sync_task', { shortId: menu.task.short_id });
              } catch (e) {
                setLastError(String(e));
              }
              setMenu(null);
            }}
          >
            Sync from Notion
          </button>
        </ContextMenu>
      )}
    </section>
  );
}

/** GitHub numbers PRs with `#`, GitLab with `!`. */
const sigil = (mr: ReviewMr) => (mr.platform === 'github' ? '#' : '!');

/** An MR waiting on the user: code = the reference, state = the project. */
function ReviewRow({ mr, busy, onOpen }: { mr: ReviewMr; busy: boolean; onOpen: () => void }) {
  return (
    <button
      className="upnext-tr is-review"
      onClick={onOpen}
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
      <span className="row-note upnext-state">{mr.project_full}</span>
      <span className="row-note upnext-owner">{busy ? 'opening…' : mr.author}</span>
    </button>
  );
}

/** A queued task: state = priority when set, else the Notion status. */
function TaskRow({ task, onMenu }: { task: Task; onMenu: (x: number, y: number) => void }) {
  return (
    <button
      className="upnext-tr"
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
        {task.priority ? (
          <span className={`prio-badge p${priorityRank(task.priority)}`}>
            {priorityLabel(task.priority)}
          </span>
        ) : (
          <span className={`row-status status-${statusKey(task.status)}`}>{task.status}</span>
        )}
      </span>
      <span className="row-note upnext-owner">—</span>
    </button>
  );
}
