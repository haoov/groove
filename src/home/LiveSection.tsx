import { useState } from 'react';
import { useStore } from '../shared/store';
import { call } from '../shared/ipc/client';
import { openExternal } from '../shared/lib/openExternal';
import { Button } from '../shared/ui';
import type { HomeEntry, HomeRepo } from '../shared/ipc/generated';

function ciClass(ci: string | null): string {
  if (ci === 'success' || ci === 'passed') return 'ci ok';
  if (ci === 'failed' || ci === 'error') return 'ci fail';
  if (ci === 'running' || ci === 'pending') return 'ci run';
  return 'ci';
}

/** Group a session's repo rows by project — several rows with the same project are
 *  several worktrees of it. */
function byProject(repos: HomeRepo[]): [string, HomeRepo[]][] {
  const map = new Map<string, HomeRepo[]>();
  for (const r of repos) {
    const list = map.get(r.project) ?? [];
    list.push(r);
    map.set(r.project, list);
  }
  return [...map.entries()];
}

function Worktree({ r }: { r: HomeRepo }) {
  const dirty = r.modified + r.staged + r.conflicted;
  return (
    <div className="wt">
      <span className="br">{r.branch ?? '—'}</span>
      {r.mr ? (
        <>
          <a className="mrlink" title="Open in GitLab" onClick={() => openExternal(r.mr!.url)}>
            !{r.mr.remote_id} ↗
          </a>
          {r.mr.ci && (
            <span className={ciClass(r.mr.ci)}><span className="d" />{r.mr.ci}</span>
          )}
          {r.mr.unresolved > 0 && <span className="thr">· {r.mr.unresolved} threads</span>}
        </>
      ) : (
        <span className="nomr">no merge request</span>
      )}
      <span className="spring" />
      {r.missing && <span className="warn">missing</span>}
      {dirty > 0 && <span className="dirty">● {dirty}</span>}
      {(r.ahead > 0 || r.behind > 0) && (
        <span className="ab">↑{r.ahead} ↓{r.behind}</span>
      )}
    </div>
  );
}

function Card({ entry }: { entry: HomeEntry }) {
  const [open, setOpen] = useState(entry.kind === 'task');
  const [armed, setArmed] = useState<'finish' | 'delete' | null>(null);
  const setView = useStore((s) => s.setView);
  const setActiveSession = useStore((s) => s.setActiveSession);
  const refreshHome = useStore((s) => s.refreshHome);

  const focus = async () => {
    try {
      await call('open_task', { shortId: entry.short_id });
      setActiveSession(entry.short_id);
      setView('session');
    } catch (e) {
      console.warn('open failed', e);
    }
  };

  const run = async (action: 'finish' | 'delete') => {
    if (armed !== action) { setArmed(action); return; }
    setArmed(null);
    try {
      await call(action === 'finish' ? 'finish_task' : 'delete_task', { shortId: entry.short_id });
      await refreshHome();
    } catch (e) {
      console.warn(action, e);
    }
  };

  return (
    <div className={`card${open ? ' open' : ''}${entry.kind === 'task' ? ' on' : ''}`}>
      <div className="row" onClick={() => setOpen((o) => !o)}>
        <span className="caret">▸</span>
        <span className={`tag ${entry.kind}`}>{entry.kind}</span>
        {entry.kind === 'task' && <span className="id">{entry.short_id}</span>}
        <span className="name">{entry.title}</span>
        <span className="spring" />
        <span className="meta">{entry.repos.length} {entry.repos.length === 1 ? 'repo' : 'repos'}</span>
        <span className="acts" onClick={(e) => e.stopPropagation()}>
          <Button variant="ghost" small onClick={focus}>Open</Button>
          {entry.kind === 'task' && (
            <Button variant={armed === 'finish' ? 'accent' : 'good'} small onClick={() => run('finish')}>
              {armed === 'finish' ? 'Confirm' : 'Finish'}
            </Button>
          )}
          <Button variant="danger" small onClick={() => run('delete')}>
            {armed === 'delete' ? 'Confirm' : entry.kind === 'task' ? 'Delete' : 'Discard'}
          </Button>
        </span>
      </div>
      {open && (
        <div className="tree">
          {entry.repos.length === 0 && <div className="empty">No repos yet.</div>}
          {byProject(entry.repos).map(([project, wts]) => (
            <div key={project}>
              <div className="repo">{project}</div>
              {wts.map((r) => (
                <Worktree key={r.worktree_id ?? r.repo_id} r={r} />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function LiveSection() {
  const snapshot = useStore((s) => s.homeSnapshot);
  return (
    <section>
      <div className="sec-h">
        <h2>Live</h2>
        <span className="desc">open sessions — click a row to focus</span>
      </div>
      {snapshot === null && <div className="loading">Loading…</div>}
      {snapshot?.length === 0 && <div className="loading">No open sessions.</div>}
      <div className="live">
        {snapshot?.map((e) => <Card key={e.short_id} entry={e} />)}
      </div>
    </section>
  );
}
