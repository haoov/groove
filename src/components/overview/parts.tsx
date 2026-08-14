import type { Mr, Repo, Worktree } from '../../types/ipc';
import { openExternal } from '../../lib/openExternal';

// Ticket bodies render through the shared Markdown component now (the backend
// converts Notion blocks to markdown), so this file only holds overview parts.

export function MrBadge({ mr }: { mr: Mr }) {
  const stateColor: Record<string, string> = {
    open:   'var(--gl-color-green-400)',
    merged: 'var(--gl-color-purple-400)',
    closed: 'var(--gl-color-red-400)',
  };
  const color = stateColor[mr.state] ?? 'var(--gl-text-color-subtle)';
  return (
    <a
      className="overview-mr"
      href={mr.url}
      onClick={(e) => { e.preventDefault(); openExternal(mr.url); }}
      style={{ borderColor: color }}
    >
      <span className="overview-mr-state" style={{ color }}>{mr.state}</span>
      <span className="overview-mr-platform">{mr.platform}</span>
      <span className="overview-mr-url">{mr.url}</span>
    </a>
  );
}

// ─── Repo row ─────────────────────────────────────────────────────────────────

export function RepoRow({ repo, worktrees }: { repo: Repo; worktrees: Worktree[] }) {
  const wt = worktrees.find((w) => w.repo_id === repo.id);
  return (
    <div className="overview-repo">
      <span className="overview-repo-name">{repo.project}</span>
      {wt && <span className="overview-repo-branch">{wt.branch}</span>}
    </div>
  );
}
