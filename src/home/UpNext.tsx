import { useMemo, useState } from 'react';
import { useStore } from '../shared/store';
import { call } from '../shared/ipc/client';
import { openExternal } from '../shared/lib/openExternal';
import { Button } from '../shared/ui';
import type { ReviewMr } from '../shared/ipc/generated';

interface Row {
  kind: 'task' | 'review';
  code: string;
  name: string;
  state: string;
  owner: string;
  open: () => void;
}

export function UpNext() {
  const tasks = useStore((s) => s.tasks);
  const queue = useStore((s) => s.reviewQueue);
  const setView = useStore((s) => s.setView);
  const setActiveSession = useStore((s) => s.setActiveSession);
  const [filter, setFilter] = useState('');
  const [showApproved, setShowApproved] = useState(false);

  const openTask = async (shortId: string) => {
    try {
      await call('open_task', { shortId });
      setActiveSession(shortId);
      setView('session');
    } catch (e) { console.warn('open_task', e); }
  };

  const openReview = async (mr: ReviewMr) => {
    if (!mr.local_path) { openExternal(mr.web_url); return; } // not cloned locally → browser
    try {
      const id = await call<string>('open_review_session', {
        projectFull: mr.project_full, iid: mr.iid, title: mr.title,
        sourceBranch: mr.source_branch, targetBranch: mr.target_branch,
        webUrl: mr.web_url, localPath: mr.local_path,
      });
      setActiveSession(id);
      setView('review');
    } catch (e) { console.warn('open_review_session', e); }
  };

  const { rows, approvedCount } = useMemo(() => {
    const taskRows: Row[] = tasks.map((t) => ({
      kind: 'task', code: t.short_id, name: t.title,
      state: t.priority ?? t.status, owner: '',
      open: () => openTask(t.short_id),
    }));
    const reviews = queue ?? [];
    const approved = reviews.filter((m) => m.approved);
    const active = reviews.filter((m) => !m.approved || showApproved);
    const reviewRows: Row[] = active.map((m) => ({
      kind: 'review', code: `!${m.iid}`, name: m.title,
      state: m.project_full, owner: m.author,
      open: () => openReview(m),
    }));
    const all = [...taskRows, ...reviewRows];
    const q = filter.trim().toLowerCase();
    const rows = q
      ? all.filter((r) => (r.name + ' ' + r.code).toLowerCase().includes(q))
      : all;
    return { rows, approvedCount: approved.length };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tasks, queue, filter, showApproved]);

  return (
    <section>
      <div className="sec-h">
        <h2>Up next</h2>
        <span className="desc">tasks from the queue + MRs to review</span>
        <span className="spring" />
        <input
          className="field"
          placeholder="Filter…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <Button variant="ghost" title="Draft a new task (composer arrives in a later slice)">Draft a task</Button>
      </div>
      <div className="queue-wrap">
        <div className="queue">
          <div className="qhead"><span>Type</span><span>Name</span><span>State</span><span>Owner</span></div>
          {rows.map((r, i) => (
            <div className="qrow" key={r.code + i} onClick={r.open}>
              <span className={`tag ${r.kind}`}>{r.kind}</span>
              <span className="nm"><span className="code">{r.code}</span>{r.name}</span>
              <span className="st">{r.state}</span>
              <span className="ow">{r.owner}</span>
            </div>
          ))}
          {rows.length === 0 && <div className="qempty">Nothing queued.</div>}
        </div>
        {approvedCount > 0 && !showApproved && (
          <div className="qmore" onClick={() => setShowApproved(true)}>
            + {approvedCount} approved {approvedCount === 1 ? 'MR' : 'MRs'} · show
          </div>
        )}
      </div>
    </section>
  );
}
