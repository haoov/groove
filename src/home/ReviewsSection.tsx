import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { ContextMenu } from '../shared/ui/ContextMenu';
import type { MainRepo, ReviewMr } from '../shared/ipc/ipc';
import { appliesTo, matchesQuery, parseQuery, type CountReport } from './filter';

// Reviews = open MRs where you are a reviewer, not yet checked out. Columns:
// id · name · repo · owner · last update.

/** The fields a Reviews row can answer — see `appliesTo`. */
const FIELDS = ['id', 'mr', 'title', 'provider', 'repo', 'branch', 'owner', 'author', 'approved', 'draft'];

const HIDDEN_KEY = 'wb.homeHiddenReviews';
function loadHidden(): Set<string> {
  try { return new Set(JSON.parse(localStorage.getItem(HIDDEN_KEY) ?? '[]')); }
  catch { return new Set(); }
}
function saveHidden(keys: Set<string>) {
  try { localStorage.setItem(HIDDEN_KEY, JSON.stringify([...keys])); } catch { /* ignore */ }
}

const sigil = (mr: ReviewMr) => (mr.platform === 'github' ? '#' : '!');

/** Rough "how long ago" for the last-update column. */
function timeAgo(iso: string): string {
  const then = new Date(iso).getTime();
  if (!Number.isFinite(then)) return '';
  const s = Math.max(0, (Date.now() - then) / 1000);
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))}m`;
  if (s < 86_400) return `${Math.round(s / 3600)}h`;
  if (s < 86_400 * 7) return `${Math.round(s / 86_400)}d`;
  return `${Math.round(s / 86_400 / 7)}w`;
}

export function ReviewsSection({ filter = '', onCount }: { filter?: string; onCount?: CountReport }) {
  const snapshot = useStore((s) => s.homeSnapshot);
  const reviewQueue = useStore((s) => s.reviewQueue);
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  const setLastError = useStore((s) => s.setLastError);
  const [busy, setBusy] = useState<string | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; mr: ReviewMr; key: string } | null>(null);
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

  // A review already open as a session belongs to Live, not here.
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
    const q = parseQuery(filter);
    const all = pending
      .filter((mr) => showApproved || !mr.approved)
      .filter((mr) => matchesQuery(q, `${sigil(mr)}${mr.iid} ${mr.title} ${mr.project_full} ${mr.author}`, {
        id: String(mr.iid),
        mr: String(mr.iid),
        title: mr.title,
        provider: mr.platform,
        repo: mr.project_full,
        branch: mr.source_branch,
        owner: mr.author,
        author: mr.author,
        approved: mr.approved,
        draft: mr.draft,
      }))
      .map((mr) => ({ mr, key: `${mr.project_full}!${mr.iid}` }));
    const hiddenN = all.filter((i) => hidden.has(i.key)).length;
    return {
      items: showHidden ? all : all.filter((i) => !hidden.has(i.key)),
      approvedHidden: showApproved ? 0 : approvedCount,
      hiddenCount: hiddenN,
    };
  }, [reviewQueue, startedReviews, filter, showApproved, hidden, showHidden]);

  const applicable = useMemo(() => appliesTo(parseQuery(filter), FIELDS), [filter]);
  useEffect(() => { onCount?.(items.length, applicable, filter); }, [items.length, applicable, filter, onCount]);

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

  return (
    <div className="upnext-root" onClick={() => setMenu(null)}>
      {(approvedHidden > 0 || showApproved || hiddenCount > 0 || showHidden) && (
        <div className="home-toolbar">
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
        </div>
      )}

      {items.length === 0 ? (
        <p className="home-empty">
          {filter.trim() ? `No review matches “${filter.trim()}”.` : 'No review requests waiting on you.'}
        </p>
      ) : (
        <div className="upnext-table reviews-table">
          <div className="upnext-head">
            <span>code</span>
            <span>name</span>
            <span>repo</span>
            <span>owner</span>
            <span>updated</span>
          </div>
          {items.map(({ mr, key }) => (
            <button
              key={key}
              className={`upnext-tr is-review${hidden.has(key) ? ' dimmed' : ''}`}
              onClick={() => openReview(mr)}
              onContextMenu={(e) => { e.preventDefault(); setMenu({ x: e.clientX, y: e.clientY, mr, key }); }}
              disabled={busy === key}
              title={`${mr.title}\n${mr.project_full}${sigil(mr)}${mr.iid}\n${mr.source_branch} → ${mr.target_branch}`}
            >
              <span className="row-key">{sigil(mr)}{mr.iid}</span>
              <span className="row-titleline">
                <span className="row-title">{mr.title}</span>
                {mr.draft && <span className="row-note">draft</span>}
                {mr.approved && <span className="approved-badge">approved</span>}
              </span>
              <span className="row-note reviews-repo">{mr.project_full}</span>
              <span className="row-note reviews-owner">{busy === key ? 'opening…' : mr.author}</span>
              <span className="row-note reviews-updated" title={`Updated ${timeAgo(mr.updated_at)} ago`}>{timeAgo(mr.updated_at)} ago</span>
            </button>
          ))}
        </div>
      )}

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} className="context-menu">
          <button className="context-item" onClick={() => { openReview(menu.mr); setMenu(null); }}>
            Open review
          </button>
          <button className="context-item" onClick={() => { toggleHidden(menu.key); setMenu(null); }}>
            {hidden.has(menu.key) ? 'Unhide' : 'Hide from Reviews'}
          </button>
        </ContextMenu>
      )}
    </div>
  );
}
