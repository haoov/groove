import { RefreshCw } from 'lucide-react';
import { invoke } from '../../shared/ipc/invoke';
import { useSession, useStore } from '../../shared/store';
import { refreshSession } from '../../shared/lib/refreshSession';
import { DIFF_MODES } from '../../shared/lib/diffModes';
import type { Worktree } from '../../shared/ipc/ipc';
import { CommitsTab } from '../../git/CommitsTab';
import { ChangedFilesList } from '../../git/ChangedFilesList';

/** The git sub-tabs: the changed-file list with its staging actions, or the commit log. */
export function GitPanel({
  repoId, activeWt, activeDirty, worktreeForRepo, onOpenFileInEditor,
}: {
  repoId: string;
  /** The active repo's worktree; scopes stage-all and discard-all. */
  activeWt?: Worktree;
  activeDirty: number;
  worktreeForRepo: (id: string) => Worktree | undefined;
  onOpenFileInEditor: (path: string, repoId: string, lang: string) => void;
}) {
  const sessionId = useSession((s) => s.id);
  const gitSubTab = useSession((s) => s.gitSubTab);
  const setGitSubTab = useSession((s) => s.setGitSubTab);
  const commits = useSession((s) => s.commits);
  const commitsHasMore = useSession((s) => s.commitsHasMore);
  const loadMoreCommits = useSession((s) => s.loadMoreCommits);
  const diffMode = useSession((s) => s.diffMode);
  const setDiffMode = useSession((s) => s.setDiffMode);
  const openTab = useSession((s) => s.openTab);
  const refreshStatus = useSession((s) => s.refreshStatus);
  const bumpDiff = useSession((s) => s.bumpDiff);
  const invalidateMrs = useStore((s) => s.invalidateMrs);
  const setLastError = useStore((s) => s.setLastError);

  // Stages one file or the whole active repo, then refreshes the status counts and the diff.
  const refreshAfterStage = () => { refreshStatus(); bumpDiff(); };
  const toggleStage = async (path: string, id: string, staged: boolean) => {
    const wt = worktreeForRepo(id);
    if (!wt) return;
    try {
      await invoke(staged ? 'stage_file' : 'unstage_file', { worktreeId: wt.id, filePath: path });
      refreshAfterStage();
    } catch (e) {
      setLastError(e);
    }
  };
  const stageAll = (cmd: 'stage_all' | 'unstage_all') => async () => {
    if (!activeWt) return;
    try {
      await invoke(cmd, { worktreeId: activeWt.id });
      refreshAfterStage();
    } catch (e) {
      setLastError(e);
    }
  };

  // Discard is destructive: it goes through the confirmation bridge.
  const discardFile = (path: string, id: string) => {
    const wt = worktreeForRepo(id);
    if (!wt) return;
    invoke('discard_file', { worktreeId: wt.id, filePath: path }).catch((e) => setLastError(e));
  };
  const discardAll = () => {
    if (!activeWt) return;
    invoke('discard_all', { worktreeId: activeWt.id }).catch((e) => setLastError(e));
  };

  return (
    <div className="git-tab">
      <div className="git-subtabs">
        {(['changes', 'commits'] as const).map((sub) => {
          const badge = sub === 'changes' ? activeDirty : 0;
          return (
            <button
              key={sub}
              className={`git-subtab ${gitSubTab === sub ? 'active' : ''}`}
              onClick={() => setGitSubTab(sub)}
            >
              {sub === 'changes' && 'Changes'}
              {sub === 'commits' && 'Commits'}
              {badge > 0 && <span className="git-subtab-badge">{badge}</span>}
            </button>
          );
        })}
      </div>

      <div className="git-subcontent">
        {gitSubTab === 'commits' && (
          <CommitsTab
            commits={commits}
            hasMore={commitsHasMore}
            onLoadMore={loadMoreCommits}
            onSelect={(c) =>
              openTab({ repoId, filePath: '', view: 'diff', kind: 'commit', sha: c.sha, label: c.short_sha })
            }
          />
        )}
        {gitSubTab === 'changes' && (
          <>
            {/* Diff base and refresh. This list and every diff tab follow the selected base. */}
            <div className="diff-mode-row">
              <div className="diff-mode-seg">
                {DIFF_MODES.map((m) => (
                  <button
                    key={m.id}
                    className={`diff-mode-btn ${diffMode === m.id ? 'active' : ''}`}
                    title={m.title}
                    onClick={() => setDiffMode(m.id)}
                  >
                    <m.Icon size={12} strokeWidth={1.75} />
                  </button>
                ))}
              </div>
              <button
                className="diff-mode-refresh"
                onClick={() => { void refreshSession(sessionId); invalidateMrs(sessionId); }}
                title="Refresh diff, git status & CI"
              >
                <RefreshCw size={12} strokeWidth={1.75} />
              </button>
            </div>
            {/* Stage-all and discard-all live on the All-changes row. */}
            <ChangedFilesList
              repoId={repoId}
              worktreeId={activeWt?.id}
              onOpenFile={(path, rid) => openTab({ repoId: rid, filePath: path, view: 'diff' })}
              onOpenFileAlt={onOpenFileInEditor}
              onOpenAll={(rid) => openTab({ repoId: rid, filePath: '', view: 'diff', kind: 'changes' })}
              onToggleStage={toggleStage}
              onDiscard={discardFile}
              onStageAll={(stage) => stageAll(stage ? 'stage_all' : 'unstage_all')()}
              onDiscardAll={discardAll}
            />
          </>
        )}
      </div>
    </div>
  );
}
