import { GitBranch, GitPullRequest } from 'lucide-react';
import type { Mr, Repo, Worktree } from '../shared/ipc/ipc';
import { openExternal } from '../shared/lib/openExternal';

// Ticket bodies render through the shared Markdown component now (the backend
// converts Notion blocks to markdown), so this file only holds overview parts.

/** One MR, beside its worktree: number + state, opening the forge. */
function MrLine({ mr }: { mr: Mr }) {
  const num = `${mr.platform === 'github' ? '#' : '!'}${mr.remote_id}`;
  return (
    <a
      className="overview-wt-mr"
      href={mr.url}
      title={`${num} — open in ${mr.platform === 'github' ? 'GitHub' : 'GitLab'}`}
      onClick={(e) => { e.preventDefault(); openExternal(mr.url); }}
    >
      <GitPullRequest size={11} strokeWidth={1.75} />
      <span className="overview-wt-mr-num">{num}</span>
      <span className={`overview-wt-mr-state mr-state-${mr.state}`}>{mr.state}</span>
    </a>
  );
}

// ─── Repo row ─────────────────────────────────────────────────────────────────

/** A repo, with each of its worktrees indented beneath it — branch on the left,
 *  the worktree's merge request (if any) beside it. */
export function RepoRow({
  repo, worktrees, mrs = [],
}: {
  repo: Repo;
  worktrees: Worktree[];
  mrs?: Mr[];
}) {
  const repoWts = worktrees.filter((w) => w.repo_id === repo.id);
  return (
    <div className="overview-repo-block">
      <div className="overview-repo">
        <span className="overview-repo-name">{repo.project}</span>
      </div>
      {repoWts.length === 0 ? (
        <div className="overview-wt overview-wt-empty">no worktree</div>
      ) : (
        repoWts.map((wt) => {
          const mr = mrs.find((m) => m.worktree_id === wt.id);
          return (
            <div className="overview-wt" key={wt.id}>
              <GitBranch size={11} strokeWidth={1.75} className="overview-wt-icon" />
              <span className="overview-wt-branch">{wt.branch}</span>
              {mr && <MrLine mr={mr} />}
            </div>
          );
        })
      )}
    </div>
  );
}
