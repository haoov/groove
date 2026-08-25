import { useState } from 'react';
import { forgeName, mrRef } from '../shared/lib/forge';
import { GitBranch, GitMerge, GitPullRequest, GitPullRequestClosed } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { openExternal } from '../shared/lib/openExternal';
import { openRepo } from './helpers';
import type { HomeEntry, HomeMr, HomeRepo } from '../shared/ipc/ipc';

/** The MR beside a worktree: number + state (+ CI dot, unresolved count). */
function MrLine({ mr }: { mr: HomeMr }) {
  const num = mrRef(mr.platform, mr.remote_id);
  const Icon = mr.state === 'merged' ? GitMerge : mr.state === 'closed' ? GitPullRequestClosed : GitPullRequest;
  return (
    <a
      className="overview-wt-mr"
      href={mr.url}
      title={`${num} ${mr.state} — open in ${forgeName(mr.platform)}`}
      onClick={(e) => { e.preventDefault(); e.stopPropagation(); openExternal(mr.url); }}
    >
      <Icon size={11} strokeWidth={1.75} />
      <span className="overview-wt-mr-num">{num}</span>
      <span className={`overview-wt-mr-state mr-state-${mr.state}`}>{mr.state}</span>
    </a>
  );
}

/** Group the flat repo+worktree rows by repo, preserving first-seen order. */
function byRepo(repos: HomeRepo[]) {
  const order: string[] = [];
  const map = new Map<string, HomeRepo[]>();
  for (const r of repos) {
    if (!map.has(r.repo_id)) { map.set(r.repo_id, []); order.push(r.repo_id); }
    map.get(r.repo_id)!.push(r);
  }
  return order.map((id) => ({ repoId: id, project: map.get(id)![0].project, worktrees: map.get(id)! }));
}

/** A live session's repos, expanded: the project name over its worktrees, each
 *  branch a clickable row (opens the editor there) with its MR beside it. */
export function LiveRepos({ entry }: { entry: HomeEntry }) {
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [working, setWorking] = useState<string | null>(null);

  // Idempotent: the same call provisions a fresh repo and re-provisions a stale one.
  const provision = async (e: React.MouseEvent, repoId: string) => {
    e.stopPropagation();
    setWorking(repoId);
    try {
      await invoke('provision_worktrees', {
        taskId: entry.short_id,
        branches: [{ repo_id: repoId, branch_name: null }],
      });
      refreshHome();
    } catch (err) {
      setLastError(String(err));
    } finally {
      setWorking(null);
    }
  };

  return (
    <div className="live-repos">
      {byRepo(entry.repos).map((g) => (
        <div className="overview-repo-block" key={g.repoId}>
          <div className="overview-repo">
            <span className="overview-repo-name">{g.project}</span>
          </div>
          {g.worktrees.map((r, i) => {
            const key = r.worktree_id ?? `${r.repo_id}-${i}`;
            if (!r.provisioned || r.missing) {
              return (
                <div className="overview-wt" key={key}>
                  <GitBranch size={11} strokeWidth={1.75} className="overview-wt-icon" />
                  <span className="overview-wt-branch muted">{r.missing ? 'missing on disk' : 'not provisioned'}</span>
                  <button className="live-wt-provision" onClick={(e) => provision(e, r.repo_id)} disabled={working === r.repo_id}>
                    {working === r.repo_id ? 'working…' : r.missing ? 're-provision' : 'provision'}
                  </button>
                </div>
              );
            }
            return (
              <div
                className="overview-wt clickable"
                key={key}
                role="button"
                tabIndex={0}
                onClick={() => openRepo(entry, r)}
                onKeyDown={(e) => { if (e.key === 'Enter') openRepo(entry, r); }}
                title={`Open ${r.branch} in the editor`}
              >
                <GitBranch size={11} strokeWidth={1.75} className="overview-wt-icon" />
                <span className="overview-wt-branch">{r.branch}</span>
                {r.mr && <MrLine mr={r.mr} />}
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}
