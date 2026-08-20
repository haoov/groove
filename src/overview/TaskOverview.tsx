import { useCallback, useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { Markdown } from '../shared/ui';
import { openExternal } from '../shared/lib/openExternal';
import { activeWorktree } from '../sessions/sessions.slice';
import type { SessionState } from '../sessions/sessions.slice';
import type { PropertyValue, TaskSchema, TimeSummary, Mr } from '../shared/ipc/generated';

const hours = (s: number) => (s / 3600).toFixed(1);

/** A task's Notion ticket: editable properties, tracked hours, an MR link, and
 *  the page body. Properties and hours write straight through (you clicked it);
 *  the MR link opens GitLab, never an in-app MR view. */
export function TaskOverview({ session }: { session: SessionState }) {
  const pageId = session.notionPageId;
  const wt = activeWorktree(session);

  const [props, setProps] = useState<PropertyValue[] | null>(null);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const [time, setTime] = useState<TimeSummary | null>(null);
  const [body, setBody] = useState<string | null>(null);
  const [mr, setMr] = useState<Mr | null>(null);

  const load = useCallback(() => {
    if (pageId) {
      call<PropertyValue[]>('get_task_properties', { notionPageId: pageId }).then(setProps).catch(() => setProps([]));
      call<string>('get_task_body_markdown', { notionPageId: pageId }).then(setBody).catch(() => setBody(''));
    }
    call<TaskSchema>('get_task_schema').then(setSchema).catch(() => {});
    call<TimeSummary>('get_task_time', { taskId: session.id }).then(setTime).catch(() => {});
    if (wt) call<Mr[]>('get_mr', { worktreeId: wt.id }).then((m) => setMr(m[0] ?? null)).catch(() => {});
  }, [pageId, session.id, wt?.id]);

  useEffect(() => { load(); }, [load]);

  const editableKinds = new Set(['select', 'status']);
  const optionsFor = (name: string) => schema?.properties.find((p) => p.name === name)?.options ?? [];
  const isEditable = (p: PropertyValue) =>
    editableKinds.has(p.kind) && (schema?.properties.find((s) => s.name === p.name)?.editable ?? false);

  const setProperty = async (name: string, value: string) => {
    if (!pageId) return;
    try {
      await call('update_task_property', { notionPageId: pageId, property: name, value });
      load();
    } catch (e) { console.warn('update_task_property failed', e); }
  };

  const [logField, setLogField] = useState('');
  const logHours = async () => {
    const h = parseFloat(logField);
    if (!pageId || !isFinite(h) || h <= 0) return;
    try {
      await call('log_task_hours', { taskId: session.id, notionPageId: pageId, hours: h });
      setLogField('');
      call<TimeSummary>('get_task_time', { taskId: session.id }).then(setTime).catch(() => {});
    } catch (e) { console.warn('log_task_hours failed', e); }
  };

  const [syncing, setSyncing] = useState(false);
  const sync = async () => {
    setSyncing(true);
    try { await call('sync_task', { shortId: session.id }); load(); }
    catch (e) { console.warn('sync_task failed', e); }
    finally { setSyncing(false); }
  };

  return (
    <div className="ovw">
      <header className="ovw-h">
        <div>
          <span className="ovw-id">{session.id}</span>
          <h1 className="ovw-title">{session.title}</h1>
        </div>
        <span className="spring" />
        {mr && <button className="ovw-link" onClick={() => openExternal(mr.url)}>MR {mr.platform === 'github' ? '#' : '!'}{mr.remote_id} ↗</button>}
        {pageId && <button className="ovw-sync" disabled={syncing} onClick={sync}>{syncing ? 'Syncing…' : 'Sync'}</button>}
      </header>

      <div className="ovw-grid">
        <section className="ovw-card">
          <h2>Properties</h2>
          {props === null && <div className="ovw-empty">Loading…</div>}
          {props?.length === 0 && <div className="ovw-empty">No properties.</div>}
          <dl className="ovw-props">
            {props?.map((p) => (
              <div key={p.name}>
                <dt>{p.name}</dt>
                <dd>
                  {isEditable(p) ? (
                    <select value={p.display} onChange={(e) => setProperty(p.name, e.target.value)}>
                      {!optionsFor(p.name).includes(p.display) && <option value={p.display}>{p.display || '—'}</option>}
                      {optionsFor(p.name).map((o) => <option key={o} value={o}>{o}</option>)}
                    </select>
                  ) : (
                    <span>{p.display || '—'}</span>
                  )}
                </dd>
              </div>
            ))}
          </dl>
        </section>

        <section className="ovw-card">
          <h2>Hours</h2>
          {time && (
            <dl className="ovw-props">
              <div><dt>Today</dt><dd>{hours(time.today_seconds)} h</dd></div>
              <div><dt>Tracked</dt><dd>{hours(time.tracked_seconds)} h</dd></div>
              <div><dt>Logged</dt><dd>{hours(time.logged_seconds)} h</dd></div>
              <div><dt>Unlogged</dt><dd>{hours(time.unlogged_seconds)} h</dd></div>
            </dl>
          )}
          {pageId && (
            <div className="ovw-log">
              <input placeholder="Hours" value={logField} inputMode="decimal"
                onChange={(e) => setLogField(e.target.value)}
                onKeyDown={(e) => { if (e.key === 'Enter') logHours(); }} />
              <button disabled={!logField.trim()} onClick={logHours}>Log to Notion</button>
            </div>
          )}
        </section>
      </div>

      <section className="ovw-card ovw-body">
        <h2>Description</h2>
        {body === null && <div className="ovw-empty">Loading…</div>}
        {body !== null && (body.trim() ? <Markdown>{body}</Markdown> : <div className="ovw-empty">Empty.</div>)}
      </section>
    </div>
  );
}
