import { useCallback, useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';
import { useStore } from '../shared/store';
import { activeWorktree } from '../sessions/sessions.slice';
import type { SessionState } from '../sessions/sessions.slice';
import type { Annotation, Mr } from '../shared/ipc/generated';

// get_mr_threads returns hand-built JSON: [{ id, notes: [note, ...] }].
interface Note { id: number; body: string; author?: { username?: string; name?: string }; resolved?: boolean; resolvable?: boolean }
interface Thread { id: string; notes: Note[] }

/** The session's review notes: local code annotations for every session, plus
 *  merge-request threads for a review. Annotations open the file they mark. */
export function NotesPanel({ session }: { session: SessionState }) {
  const isReview = session.kind === 'review';
  return (
    <aside className="sidebar notes-panel">
      <div className="side-h">Notes</div>
      <div className="notes-body">
        <Annotations session={session} />
        {isReview && <MrThreads session={session} />}
      </div>
    </aside>
  );
}

function Annotations({ session }: { session: SessionState }) {
  const openFileTab = useStore((s) => s.openFileTab);
  const [items, setItems] = useState<Annotation[] | null>(null);

  const load = useCallback(() => {
    call<Annotation[]>('get_annotations', { sessionId: session.id, repoId: null })
      .then(setItems).catch(() => setItems([]));
  }, [session.id]);

  useEffect(() => {
    load();
    // The agent creates/resolves annotations too; refetch when it does.
    const uns = [on(EV.annotationCreated, load), on(EV.annotationResolved, load)];
    return () => { uns.forEach((p) => p.then((u) => u())); };
  }, [load]);

  const resolve = async (id: string) => {
    try { await call('resolve_annotation', { id }); load(); } catch (e) { console.warn('resolve_annotation failed', e); }
  };
  const remove = async (id: string) => {
    try { await call('delete_annotation', { id }); load(); } catch (e) { console.warn('delete_annotation failed', e); }
  };

  const open = ({ repo_id, file_path }: Annotation) => openFileTab(session.id, repo_id, file_path);
  const lineLabel = (a: Annotation) => (a.start_line === a.end_line ? `${a.start_line}` : `${a.start_line}–${a.end_line}`);

  return (
    <section className="notes-sec">
      <h3>Annotations</h3>
      {items === null && <div className="notes-empty">Loading…</div>}
      {items?.length === 0 && <div className="notes-empty">No annotations. The agent and you leave them on code.</div>}
      {items?.map((a) => (
        <div key={a.id} className={`note${a.status === 'resolved' ? ' resolved' : ''}`}>
          <div className="note-loc" onClick={() => open(a)}>
            <span className="note-file">{a.file_path.split('/').pop()}</span>
            <span className="note-line">:{lineLabel(a)}</span>
          </div>
          <div className="note-body">{a.content}</div>
          <div className="note-foot">
            <span className="note-author">{a.author}</span>
            <span className="spring" />
            {a.status !== 'resolved' && <button onClick={() => resolve(a.id)}>Resolve</button>}
            <button onClick={() => remove(a.id)}>Delete</button>
          </div>
        </div>
      ))}
    </section>
  );
}

function MrThreads({ session }: { session: SessionState }) {
  const wt = activeWorktree(session);
  const [mrId, setMrId] = useState<string | null>(null);
  const [threads, setThreads] = useState<Thread[] | null>(null);
  const [comment, setComment] = useState('');
  const [replyOn, setReplyOn] = useState<string | null>(null);
  const [replyText, setReplyText] = useState('');

  const load = useCallback(async () => {
    if (!wt) return;
    try {
      const mrs = await call<Mr[]>('get_mr', { worktreeId: wt.id });
      const id = mrs[0]?.id ?? null;
      setMrId(id);
      if (id) setThreads(await call<Thread[]>('get_mr_threads', { mrId: id }));
    } catch (e) { console.warn('get_mr_threads failed', e); }
  }, [wt?.id]);

  useEffect(() => { load(); }, [load]);

  const author = (n: Note) => n.author?.username || n.author?.name || 'unknown';
  const unresolved = (t: Thread) => t.notes[0]?.resolvable === true && t.notes[0]?.resolved === false;

  const postComment = async () => {
    if (!mrId || !comment.trim()) return;
    try { await call('post_mr_comment', { mrId, body: comment.trim(), filePath: null, line: null }); setComment(''); load(); }
    catch (e) { console.warn('post_mr_comment failed', e); }
  };
  const reply = async (threadId: string) => {
    if (!mrId || !replyText.trim()) return;
    try { await call('reply_to_thread', { mrId, threadId, body: replyText.trim() }); setReplyText(''); setReplyOn(null); load(); }
    catch (e) { console.warn('reply_to_thread failed', e); }
  };
  const resolveThread = async (threadId: string) => {
    if (!mrId) return;
    try { await call('resolve_mr_thread', { mrId, threadId }); load(); }
    catch (e) { console.warn('resolve_mr_thread failed', e); }
  };

  return (
    <section className="notes-sec">
      <h3>MR threads</h3>
      {threads === null && <div className="notes-empty">Loading…</div>}
      {threads?.length === 0 && <div className="notes-empty">No threads.</div>}
      {threads?.map((t) => (
        <div key={t.id} className={`thread${unresolved(t) ? ' unresolved' : ''}`}>
          {t.notes.map((n) => (
            <div key={n.id} className="tnote">
              <div className="tnote-author">{author(n)}</div>
              <div className="tnote-body">{n.body}</div>
            </div>
          ))}
          <div className="thread-foot">
            {replyOn === t.id ? (
              <div className="thread-reply">
                <input autoFocus placeholder="Reply…" value={replyText}
                  onChange={(e) => setReplyText(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Enter') reply(t.id); }} />
                <button onClick={() => reply(t.id)}>Send</button>
              </div>
            ) : (
              <>
                <button onClick={() => { setReplyOn(t.id); setReplyText(''); }}>Reply</button>
                {unresolved(t) && <button onClick={() => resolveThread(t.id)}>Resolve</button>}
              </>
            )}
          </div>
        </div>
      ))}
      <div className="notes-new">
        <textarea rows={2} placeholder="New comment on the MR…" value={comment}
          onChange={(e) => setComment(e.target.value)} />
        <button disabled={!comment.trim()} onClick={postComment}>Comment</button>
      </div>
    </section>
  );
}
