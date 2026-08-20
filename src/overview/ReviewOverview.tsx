import { useCallback, useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { Markdown } from '../shared/ui';
import { openExternal } from '../shared/lib/openExternal';
import { ciTone } from './ci';
import { activeWorktree } from '../sessions/sessions.slice';
import type { SessionState } from '../sessions/sessions.slice';
import type { Mr } from '../shared/ipc/generated';

// get_mr_details / get_mr_ci return hand-built JSON (no ts-rs DTO).
interface MrDetails {
  title: string; description: string; author: string;
  source_branch: string; target_branch: string; state: string; draft: boolean;
  web_url: string; approved: boolean; approved_by_me: boolean; approved_by: string[];
}
interface Ci { status: string; url: string }

/** A review session's merge request: state, CI, approvals, and the description,
 *  with direct edit/approve actions. The MR link always opens the forge. */
export function ReviewOverview({ session }: { session: SessionState }) {
  const wt = activeWorktree(session);
  const [mr, setMr] = useState<Mr | null>(null);
  const [details, setDetails] = useState<MrDetails | null>(null);
  const [ci, setCi] = useState<Ci | null>(null);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState({ title: '', description: '' });
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!wt) return;
    try {
      const mrs = await call<Mr[]>('get_mr', { worktreeId: wt.id });
      const first = mrs[0] ?? null;
      setMr(first);
      if (!first) return;
      const [d, c] = await Promise.all([
        call<MrDetails>('get_mr_details', { mrId: first.id }),
        call<Ci | null>('get_mr_ci', { mrId: first.id }),
      ]);
      setDetails(d);
      setCi(c);
    } catch (e) { console.warn('review overview load failed', e); }
  }, [wt?.id]);

  useEffect(() => { load(); }, [load]);

  const tag = mr?.platform === 'github' ? `#${mr.remote_id}` : `!${mr?.remote_id ?? ''}`;

  const approve = async () => {
    if (!mr) return;
    setBusy(true);
    try { await call('approve_mr', { mrId: mr.id }); await load(); }
    catch (e) { console.warn('approve_mr failed', e); }
    finally { setBusy(false); }
  };

  const startEdit = () => {
    if (!details) return;
    setDraft({ title: details.title, description: details.description });
    setEditing(true);
  };
  const saveEdit = async () => {
    if (!mr) return;
    setBusy(true);
    try {
      await call('edit_mr_text', { mrId: mr.id, title: draft.title, description: draft.description });
      setEditing(false);
      await load();
    } catch (e) { console.warn('edit_mr_text failed', e); }
    finally { setBusy(false); }
  };

  if (!mr && wt) return <div className="ovw"><div className="ovw-empty">Loading merge request…</div></div>;
  if (!mr) return <div className="ovw"><div className="ovw-empty">No merge request for this review.</div></div>;

  return (
    <div className="ovw">
      <header className="ovw-h">
        <div>
          <span className="ovw-id">{session.title.includes(tag) ? '' : tag}</span>
          <h1 className="ovw-title">{details?.title ?? session.title}</h1>
        </div>
        <span className="spring" />
        <button className="ovw-link" onClick={() => openExternal(details?.web_url ?? mr.url)}>Open in forge ↗</button>
      </header>

      <div className="ovw-badges">
        {details?.draft && <span className="badge b-muted">Draft</span>}
        {details && <span className={`badge b-${details.state === 'merged' ? 'good' : details.state === 'closed' ? 'bad' : 'open'}`}>{details.state}</span>}
        {ci && <span className={`badge ci-${ciTone(ci.status)}`} onClick={() => openExternal(ci.url)} role="button">CI {ci.status}</span>}
        {details?.approved && <span className="badge b-good">Approved</span>}
      </div>

      <div className="ovw-grid">
        <section className="ovw-card">
          <h2>Branches</h2>
          <dl className="ovw-props">
            <div><dt>Source</dt><dd>{details?.source_branch}</dd></div>
            <div><dt>Target</dt><dd>{details?.target_branch}</dd></div>
            <div><dt>Author</dt><dd>{details?.author}</dd></div>
          </dl>
        </section>

        <section className="ovw-card">
          <h2>Approvals</h2>
          <div className="ovw-approvers">
            {details?.approved_by?.length
              ? details.approved_by.map((u) => <span key={u} className="chip">{u}</span>)
              : <span className="ovw-empty">No approvals yet.</span>}
          </div>
          <button className="ovw-approve" disabled={busy || details?.approved_by_me} onClick={approve}>
            {details?.approved_by_me ? 'Approved by you' : 'Approve'}
          </button>
        </section>
      </div>

      <section className="ovw-card ovw-body">
        <div className="ovw-body-h">
          <h2>Description</h2>
          <span className="spring" />
          {editing
            ? <><button className="ovw-ghost" onClick={() => setEditing(false)}>Cancel</button><button className="ovw-approve" disabled={busy} onClick={saveEdit}>Save</button></>
            : <button className="ovw-ghost" onClick={startEdit}>Edit</button>}
        </div>
        {editing ? (
          <div className="ovw-edit">
            <input value={draft.title} onChange={(e) => setDraft((d) => ({ ...d, title: e.target.value }))} placeholder="Title" />
            <textarea rows={10} value={draft.description} onChange={(e) => setDraft((d) => ({ ...d, description: e.target.value }))} placeholder="Description" />
          </div>
        ) : details?.description?.trim() ? (
          <Markdown>{details.description}</Markdown>
        ) : <div className="ovw-empty">No description.</div>}
      </section>
    </div>
  );
}
