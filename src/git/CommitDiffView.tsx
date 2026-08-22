import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { GitCommit } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { worktreeFor } from '../shared/lib/workspace';
import type { FileDiff, Hunk } from '../shared/ipc/ipc';
import { FileDiffEditor } from '../editor/FileDiffEditor';
import { useDiffExpand } from '../editor/useDiffExpand';
import type { AnnCtx } from '../editor/useAnnotations';

// Commits are immutable, so their diffs cache safely across tab switches
// (tab bodies unmount when inactive). Small LRU-ish cap keeps memory bounded.
const commitDiffCache = new Map<string, FileDiff[]>();
const CACHE_MAX = 24;
function cachePut(key: string, files: FileDiff[]) {
  if (commitDiffCache.size >= CACHE_MAX) {
    const oldest = commitDiffCache.keys().next().value;
    if (oldest !== undefined) commitDiffCache.delete(oldest);
  }
  commitDiffCache.set(key, files);
}

/** One commit's diff — the "All changes" layout, minus everything that only
 *  makes sense against the working tree (annotations, staging). */
export function CommitDiffView({ repoId, sha, ann }: { repoId: string; sha: string; ann: AnnCtx }) {
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const commits = useSession((s) => s.commits);
  const setLastError = useStore((s) => s.setLastError);

  const wt = worktreeFor(activeWorktrees, repoId);
  const cacheKey = wt ? `${wt.id}:${sha}` : sha;

  const [files, setFiles] = useState<FileDiff[] | null>(() => commitDiffCache.get(cacheKey) ?? null);
  const [error, setError] = useState<string | null>(null);
  // Default expanded: a commit is a finished snapshot — you came to read it.
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());

  const meta = useMemo(() => commits.find((c) => c.sha === sha) ?? null, [commits, sha]);

  useEffect(() => {
    if (!wt || files !== null) return;
    let stale = false;
    invoke<FileDiff[]>('get_commit_diff', { worktreeId: wt.id, sha })
      .then((f) => {
        if (stale) return;
        cachePut(cacheKey, f);
        setFiles(f);
      })
      .catch((e) => {
        if (stale) return;
        setError(String(e));
        setLastError(String(e));
      });
    return () => { stale = true; };
  }, [wt, sha, cacheKey, files, setLastError]);

  const toggle = (path: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path); else next.add(path);
      return next;
    });

  if (!wt) return <div className="diff-empty"><p>No worktree for this repo</p></div>;

  return (
    <div className="diff-view changes-view">
      <div className="commit-view-header">
        <div className="commit-view-title">
          <GitCommit size={14} strokeWidth={1.75} className="commit-icon" />
          <span className="commit-sha">{sha.slice(0, 7)}</span>
          <span className="commit-view-msg">{meta?.message ?? ''}</span>
        </div>
        {meta && (
          <div className="commit-view-meta">
            <span>{meta.author}</span>
            <span className="commit-view-dot">·</span>
            <span>{new Date(meta.timestamp * 1000).toLocaleString()}</span>
          </div>
        )}
      </div>

      {error ? (
        <div className="diff-empty"><p>{error}</p></div>
      ) : files === null ? (
        <div className="diff-file-loading">Loading commit…</div>
      ) : files.length === 0 ? (
        <div className="diff-empty"><p>Empty commit</p></div>
      ) : (
        <div className="diff-repo">
          {files.map((f) => {
            const isCollapsed = collapsed.has(f.path);
            return (
              <div key={f.path} className="diff-file">
                <div className="diff-file-header" onClick={() => toggle(f.path)}>
                  <span className="diff-expand">{isCollapsed ? '▸' : '▾'}</span>
                  <span className="diff-file-path">{f.path}</span>
                  <span className="diff-repo-stats">
                    {f.added > 0 && <span className="diff-add">+{f.added}</span>}
                    {f.deleted > 0 && <span className="diff-del">−{f.deleted}</span>}
                  </span>
                </div>
                {!isCollapsed && (
                  <div className="diff-hunks">
                    {f.hunks.length === 0 ? (
                      <div className="diff-file-loading">Binary file (or no text changes)</div>
                    ) : (
                      <CommitFileDiff
                        file={f}
                        worktreeId={wt.id}
                        repoId={repoId}
                        sha={sha}
                        ann={ann}
                        onHunks={(hunks) => setFiles((prev) => {
                          if (!prev) return prev;
                          const next = prev.map((p) => (p.path === f.path ? { ...p, hunks } : p));
                          cachePut(cacheKey, next);
                          return next;
                        })}
                      />
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** One file of a commit. Context comes from that commit, not the working tree, so
 *  the expansion is keyed on the sha. */
function CommitFileDiff({
  file, worktreeId, repoId, sha, ann, onHunks,
}: {
  file: FileDiff;
  worktreeId: string;
  repoId: string;
  sha: string;
  ann: AnnCtx;
  onHunks: (hunks: Hunk[]) => void;
}) {
  const expand = useDiffExpand({
    worktreeId, filePath: file.path, hunks: file.hunks, onHunks, rev: sha,
  });

  return (
    <FileDiffEditor
      hunks={file.hunks}
      filePath={file.path}
      repoId={repoId}
      ann={ann}
      sel={null}
      dragRange={null}
      fileAnnotations={[]}
      threads={[]}
      mr={null}
      allowAnnotations={false}
      onExpandGap={expand.onExpand}
      fileLineCount={expand.total}
    />
  );
}
