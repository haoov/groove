import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore } from '../shared/store';
import { ciGroup } from '../notes/MrThreads';
import { openRepo } from './helpers';
import type { HomeEntry, HomeRepo } from '../shared/ipc/ipc';

/** One repo of a live session: branch + working-tree stats, review progress, MR. */
export function RepoRow({ entry, repo }: { entry: HomeEntry; repo: HomeRepo }) {
  const refreshHome = useStore((s) => s.refreshHome);
  const setLastError = useStore((s) => s.setLastError);
  const [working, setWorking] = useState(false);

  // Same call for a never-provisioned repo and a stale one whose folder was
  // deleted — provisioning is idempotent and prunes the stale registration.
  const provision = async (e: React.MouseEvent) => {
    e.stopPropagation();
    setWorking(true);
    try {
      await invoke('provision_worktrees', {
        taskId: entry.short_id,
        branches: [{ repo_id: repo.repo_id, branch_name: null }],
      });
      refreshHome();
    } catch (err) {
      setLastError(String(err));
    } finally {
      setWorking(false);
    }
  };

  if (!repo.provisioned || repo.missing) {
    return (
      <div className="detail-row">
        <span className="detail-repo">{repo.project}</span>
        <span className={repo.missing ? 'stat-warn' : 'muted'}>
          {repo.missing ? 'missing on disk' : 'not provisioned'}
        </span>
        <button className="home-link" onClick={provision} disabled={working}>
          {working ? 'working…' : repo.missing ? 're-provision' : 'provision'}
        </button>
      </div>
    );
  }

  const clean = repo.modified === 0 && repo.staged === 0 && repo.added === 0 && repo.deleted === 0;

  return (
    <>
      <div
        className="detail-row clickable"
        role="button"
        tabIndex={0}
        onClick={() => openRepo(entry, repo)}
        onKeyDown={(e) => { if (e.key === 'Enter') openRepo(entry, repo); }}
      >
        <span className="detail-repo">{repo.project}</span>
        <span className="detail-branch">{repo.branch}</span>
        <span className="detail-stats">
          {clean ? (
            <span className="muted">clean</span>
          ) : (
            <>
              {repo.added > 0 && <span className="stat-add">+{repo.added}</span>}
              {repo.deleted > 0 && <span className="stat-del">−{repo.deleted}</span>}
              {repo.modified > 0 && <span className="stat-dirty" title={`${repo.modified} modified`}>~{repo.modified}</span>}
              {repo.staged > 0 && <span className="stat-staged" title={`${repo.staged} staged`}>●{repo.staged}</span>}
              {repo.conflicted > 0 && <span className="stat-bad" title={`${repo.conflicted} conflicted`}>!{repo.conflicted}</span>}
            </>
          )}
          {repo.ahead > 0 && <span className="stat-ahead" title={`${repo.ahead} to push`}>↑{repo.ahead}</span>}
          {repo.behind > 0 && <span className="stat-behind" title={`${repo.behind} behind`}>↓{repo.behind}</span>}
        </span>
      </div>


      {repo.mr && (
        <div className="detail-row detail-mr-row">
          <span className="detail-repo" />
          <span className="detail-mr">
            <span className="detail-mr-num">
              {repo.mr.platform === 'github' ? '#' : '!'}{repo.mr.remote_id}
            </span>
            <span className={`mr-state-${repo.mr.state}`}>{repo.mr.state}</span>
            {repo.mr.approved && (
              <span className="approved-badge" title="Approved, not merged yet">approved</span>
            )}
            {repo.mr.ci && (
              <span className={`ci-${ciGroup(repo.mr.ci)}`}>{repo.mr.ci.replace(/_/g, ' ')}</span>
            )}
            {repo.mr.unresolved > 0 && (
              <span className="stat-threads">{repo.mr.unresolved} unresolved</span>
            )}
          </span>
        </div>
      )}
    </>
  );
}
