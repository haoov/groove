import { GitBranch, GitPullRequest } from 'lucide-react';
import { forgeName, mrRef } from '../shared/lib/forge';
import type { Mr, Repo, Worktree } from '../shared/ipc/ipc';
import { openExternal } from '../shared/lib/openExternal';

// Ticket bodies render through the shared Markdown component now (the backend
// renders the task body), so this file only holds overview parts.

/** One MR, beside its worktree: number + state, opening the forge. */
function MrLine({ mr }: { mr: Mr }) {
  const num = mrRef(mr.platform, mr.remote_id);
  return (
    <a
      className="overview-wt-mr"
      href={mr.url}
      title={`${num} — open in ${forgeName(mr.platform)}`}
      // Stop the click reaching the worktree row (which opens the editor).
      onClick={(e) => { e.preventDefault(); e.stopPropagation(); openExternal(mr.url); }}
    >
      <GitPullRequest size={11} strokeWidth={1.75} />
      <span className="overview-wt-mr-num">{num}</span>
      <span className={`overview-wt-mr-state mr-state-${mr.state}`}>{mr.state}</span>
    </a>
  );
}

// ─── Repo row ─────────────────────────────────────────────────────────────────

/** A repo, with each of its worktrees indented beneath it — branch on the left,
 *  the worktree's merge request (if any) beside it. Clicking a worktree opens
 *  the editor scoped to that repo + branch. */
export function RepoRow({
  repo, worktrees, mrs = [], onOpenWorktree,
}: {
  repo: Repo;
  worktrees: Worktree[];
  mrs?: Mr[];
  onOpenWorktree?: (repoId: string, worktreeId: string) => void;
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
          const open = onOpenWorktree ? () => onOpenWorktree(repo.id, wt.id) : undefined;
          return (
            <div
              className={`overview-wt${open ? ' clickable' : ''}`}
              key={wt.id}
              role={open ? 'button' : undefined}
              tabIndex={open ? 0 : undefined}
              onClick={open}
              onKeyDown={open ? (e) => { if (e.key === 'Enter') open(); } : undefined}
              title={open ? `Open ${wt.branch} in the editor` : undefined}
            >
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
