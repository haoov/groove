import { useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { DiffLines } from './DiffLines';
import type { SessionState, DiffMode } from '../sessions/sessions.slice';
import type { DiffResult, Hunk, FileDiff } from '../shared/ipc/generated';

const MODES: DiffMode[] = ['vs-main', 'vs-remote', 'working'];
const MODE_LABEL: Record<DiffMode, string> = { 'vs-main': 'vs main', 'vs-remote': 'vs remote', working: 'working' };

/** The "All changes" review for a session: every repo's changed files, each
 *  expandable to its diff, under a base-mode toggle. */
export function ChangesView({ session }: { session: SessionState }) {
  const setDiffMode = useStore((s) => s.setDiffMode);
  const bumpDiff = useStore((s) => s.bumpDiff);
  const mode = session.diffMode;

  const [result, setResult] = useState<DiffResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [hunks, setHunks] = useState<Record<string, Hunk[] | null>>({});

  useEffect(() => {
    setLoading(true);
    setResult(null);
    setHunks({});
    call<DiffResult>('get_task_diff_summary', { taskId: session.id, mode })
      .then(setResult)
      .catch((e) => console.warn('get_task_diff_summary failed', e))
      .finally(() => setLoading(false));
  }, [session.id, mode, session.diffNonce]);

  const worktreeIdFor = (repoId: string, branch: string) =>
    session.worktrees.find((w) => w.repo_id === repoId && w.branch === branch)?.id ??
    session.worktrees.find((w) => w.repo_id === repoId)?.id ?? null;

  const toggleFile = async (repoId: string, branch: string, file: FileDiff) => {
    const key = `${repoId}::${file.path}`;
    setOpen((s) => { const n = new Set(s); n.has(key) ? n.delete(key) : n.add(key); return n; });
    if (hunks[key] !== undefined) return;
    const wtId = worktreeIdFor(repoId, branch);
    if (!wtId) return;
    setHunks((h) => ({ ...h, [key]: null }));
    try {
      const hs = await call<Hunk[]>('get_file_diff', { worktreeId: wtId, filePath: file.path, mode });
      setHunks((h) => ({ ...h, [key]: hs }));
    } catch (e) { console.warn('get_file_diff failed', e); }
  };

  const totalFiles = result?.repos.reduce((n, r) => n + r.files.length, 0) ?? 0;

  return (
    <div className="changes">
      <div className="changes-top">
        <div className="dseg">
          {MODES.map((m) => (
            <button key={m} className={m === mode ? 'on' : ''} onClick={() => setDiffMode(session.id, m)}>
              {MODE_LABEL[m]}
            </button>
          ))}
        </div>
        <span className="spring" />
        <span className="count">{totalFiles} {totalFiles === 1 ? 'file' : 'files'}</span>
        <button className="refresh" title="Refresh" onClick={() => bumpDiff(session.id)}>↻</button>
      </div>
      <div className="changes-body">
        {loading && <div className="diff-empty">Loading…</div>}
        {!loading && totalFiles === 0 && <div className="diff-empty">No changes against the {MODE_LABEL[mode]} base.</div>}
        {result?.repos.map((repo) => (
          <div key={repo.repo_id + repo.branch} className="crepo">
            {result.repos.length > 1 && <div className="crepo-h">{repo.repo_id} · {repo.branch}</div>}
            {repo.files.map((f) => {
              const key = `${repo.repo_id}::${f.path}`;
              const isOpen = open.has(key);
              return (
                <div key={key} className="cfile">
                  <div className="cfile-h" onClick={() => toggleFile(repo.repo_id, repo.branch, f)}>
                    <span className={`caret${isOpen ? ' open' : ''}`}>▸</span>
                    <span className={`stat s-${f.status}`}>{f.status}</span>
                    <span className="fpath">{f.path}</span>
                    <span className="spring" />
                    <span className="adds">+{f.added}</span>
                    <span className="dels">−{f.deleted}</span>
                  </div>
                  {isOpen && (
                    hunks[key] === undefined || hunks[key] === null
                      ? <div className="diff-empty">Loading…</div>
                      : <DiffLines hunks={hunks[key]!} />
                  )}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
