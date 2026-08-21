import { GitPullRequest } from 'lucide-react';
import type { Mr, Repo, Worktree } from '../shared/ipc/ipc';
import { openExternal } from '../shared/lib/openExternal';

// Ticket bodies render through the shared Markdown component now (the backend
// converts Notion blocks to markdown), so this file only holds overview parts.

/** One MR line under its repo: number + state, opening the forge. */
function MrLine({ mr }: { mr: Mr }) {
  const num = `${mr.platform === 'github' ? '#' : '!'}${mr.remote_id}`;
  return (
    <a
      className="overview-repo-mr"
      href={mr.url}
      title={`${num} — open in ${mr.platform === 'github' ? 'GitHub' : 'GitLab'}`}
      onClick={(e) => { e.preventDefault(); openExternal(mr.url); }}
    >
      <GitPullRequest size={11} strokeWidth={1.75} />
      <span className="overview-repo-mr-num">{num}</span>
      <span className={`overview-repo-mr-state mr-state-${mr.state}`}>{mr.state}</span>
    </a>
  );
}

// ─── Repo row ─────────────────────────────────────────────────────────────────

/** A repo with its branch, and its merge requests listed beneath it. */
export function RepoRow({
  repo, worktrees, mrs = [],
}: {
  repo: Repo;
  worktrees: Worktree[];
  mrs?: Mr[];
}) {
  const wt = worktrees.find((w) => w.repo_id === repo.id);
  return (
    <div className="overview-repo-block">
      <div className="overview-repo">
        <span className="overview-repo-name">{repo.project}</span>
        {wt && <span className="overview-repo-branch">{wt.branch}</span>}
      </div>
      {mrs.map((mr) => <MrLine key={mr.id} mr={mr} />)}
    </div>
  );
}
