import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { call } from '../shared/ipc/client';
import { on, EV } from '../shared/ipc/events';
import { useStore } from '../shared/store';
import { activeWorktree } from '../sessions/sessions.slice';
import type { SessionState } from '../sessions/sessions.slice';
import type { DiffResult, FileDiff, WorktreeStatus } from '../shared/ipc/generated';

/** Source-control panel for the active worktree: staged/unstaged file lists
 *  with stage/unstage/discard, a commit box, and branch push/pull/rebase.
 *  Destructive and remote actions (commit, discard, push, pull, rebase) go
 *  through the approval bridge; direct staging refreshes immediately. */
export function GitPanel({ session }: { session: SessionState }) {
  const wt = activeWorktree(session);
  const bumpDiff = useStore((s) => s.bumpDiff);

  const [status, setStatus] = useState<WorktreeStatus | null>(null);
  const [files, setFiles] = useState<FileDiff[] | null>(null);
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (!wt) { setFiles([]); setStatus(null); return; }
    try {
      const [st, diff] = await Promise.all([
        call<WorktreeStatus>('get_worktree_status', { worktreeId: wt.id }),
        call<DiffResult>('get_task_diff_summary', { taskId: session.id, mode: 'working' }),
      ]);
      setStatus(st);
      const repo = diff.repos.find((r) => r.branch === wt.branch) ?? diff.repos[0];
      setFiles(repo?.files ?? []);
    } catch (e) {
      console.warn('git status refresh failed', e);
      setFiles([]);
    }
  }, [wt?.id, wt?.branch, session.id]);

  useEffect(() => { setFiles(null); refresh(); }, [refresh]);

  // The approval bridge resolves commit/discard/push/pull/rebase off-thread;
  // refetch when any of those land so the panel reflects the new state.
  useEffect(() => {
    const uns = [
      on(EV.confirmationResolved, () => refresh()),
      on(EV.rebaseDone, () => refresh()),
    ];
    return () => { uns.forEach((p) => p.then((u) => u())); };
  }, [refresh]);

  if (!wt) {
    return <aside className="sidebar"><div className="side-h">Source control</div><div className="tempty">No worktree in this session.</div></aside>;
  }

  const staged = (files ?? []).filter((f) => f.staged === true);
  const unstaged = (files ?? []).filter((f) => f.staged !== true);

  // Direct ops resolve immediately; bridge ops return a request id and resolve later.
  const run = async (p: Promise<unknown>) => {
    setBusy(true);
    try { await p; } catch (e) { console.warn('git op failed', e); }
    finally { setBusy(false); await refresh(); bumpDiff(session.id); }
  };

  const stageFile = (path: string) => run(call('stage_file', { worktreeId: wt.id, filePath: path }));
  const unstageFile = (path: string) => run(call('unstage_file', { worktreeId: wt.id, filePath: path }));
  const discardFile = (path: string) => run(call('discard_file', { worktreeId: wt.id, filePath: path }));
  const stageAll = () => run(call('stage_all', { worktreeId: wt.id }));
  const unstageAll = () => run(call('unstage_all', { worktreeId: wt.id }));

  const commit = () => {
    const m = message.trim();
    if (!m) return;
    run(call('commit', { worktreeId: wt.id, message: m })).then(() => setMessage(''));
  };
  const push = () => run(call('push', { worktreeId: wt.id }));
  const pull = () => run(call('pull', { worktreeId: wt.id }));
  const rebase = () => run(call('rebase_on_main', { worktreeId: wt.id }));

  const total = (files ?? []).length;

  return (
    <aside className="sidebar git-panel">
      <div className="side-h">Source control</div>

      <div className="gbranch">
        <span className="gbranch-name">{wt.branch}</span>
        {status && (
          <span className="gbranch-track">
            {status.ahead > 0 && <span title="commits to push">↑{status.ahead}</span>}
            {status.behind > 0 && <span title="commits to pull">↓{status.behind}</span>}
          </span>
        )}
      </div>

      <div className="gcommit">
        <textarea
          placeholder="Commit message"
          value={message}
          rows={2}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) commit(); }}
        />
        <button className="gcommit-btn" disabled={busy || !message.trim() || total === 0} onClick={commit}>
          Commit{staged.length > 0 ? ` (${staged.length} staged)` : total > 0 ? ' all' : ''}
        </button>
      </div>

      <div className="glist">
        {files === null && <div className="tempty">Loading…</div>}
        {files?.length === 0 && <div className="tempty">No changes.</div>}

        {staged.length > 0 && (
          <GitSection title="Staged" count={staged.length} action="Unstage all" onAction={unstageAll} busy={busy}>
            {staged.map((f) => (
              <GitRow key={f.path} file={f} busy={busy}
                actions={[['−', 'Unstage', () => unstageFile(f.path)]]} />
            ))}
          </GitSection>
        )}

        {unstaged.length > 0 && (
          <GitSection title="Changes" count={unstaged.length} action="Stage all" onAction={stageAll} busy={busy}>
            {unstaged.map((f) => (
              <GitRow key={f.path} file={f} busy={busy}
                actions={[['↩', 'Discard', () => discardFile(f.path)], ['+', 'Stage', () => stageFile(f.path)]]} />
            ))}
          </GitSection>
        )}
      </div>

      <div className="gactions">
        <button disabled={busy} onClick={push} title="Push this branch">Push</button>
        <button disabled={busy} onClick={pull} title="Pull with rebase">Pull</button>
        <button disabled={busy} onClick={rebase} title="Rebase onto the base branch">Rebase</button>
      </div>
    </aside>
  );
}

type RowAction = [glyph: string, title: string, run: () => void];

function GitSection({
  title, count, action, onAction, busy, children,
}: {
  title: string; count: number; action: string; onAction: () => void; busy: boolean; children: ReactNode;
}) {
  return (
    <div className="gsection">
      <div className="gsection-h">
        <span>{title} <i>{count}</i></span>
        <button className="glink" disabled={busy} onClick={onAction}>{action}</button>
      </div>
      {children}
    </div>
  );
}

function GitRow({ file, actions, busy }: { file: FileDiff; actions: RowAction[]; busy: boolean }) {
  const name = file.path.split('/').pop() ?? file.path;
  const dir = file.path.slice(0, file.path.length - name.length);
  return (
    <div className="grow">
      <span className={`stat s-${file.status}`}>{file.status}</span>
      <span className="gname" title={file.path}>{name}</span>
      <span className="gdir">{dir}</span>
      <span className="spring" />
      <span className="grow-actions">
        {actions.map(([glyph, title, run]) => (
          <button key={title} title={title} disabled={busy} onClick={run}>{glyph}</button>
        ))}
      </span>
    </div>
  );
}
