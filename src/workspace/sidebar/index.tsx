import { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RotateCcw, RefreshCw } from 'lucide-react';
import { useStore, useSession } from '../../shared/store';
import { DIFF_MODES } from '../../git/diffModes';
import type { CommitEntry } from '../../shared/ipc/ipc';
import { countUnresolved } from '../useWorkspaceData';
import { RepoSwitcher } from './RepoSwitcher';
import { FilesTab } from '../../files/FilesTab';
import { CommitsTab, ChangedFilesList, GitCommitPanel } from '../../git/GitTab';
import { ForgeSection } from '../../git/ForgeSection';
import { AnnotationsTab } from '../../notes/AnnotationsTab';

export function Sidebar() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const sidebarTab = useSession((s) => s.sidebarTab);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const worktreeStatus = useSession((s) => s.worktreeStatus);
  const refreshStatus = useSession((s) => s.refreshStatus);
  const bumpDiff = useSession((s) => s.bumpDiff);
  const bumpMrs = useSession((s) => s.bumpMrs);
  const diffMode = useSession((s) => s.diffMode);
  const setDiffMode = useSession((s) => s.setDiffMode);
  const diff = useSession((s) => s.diff);
  const annotations = useSession((s) => s.annotations);
  const commits = useSession((s) => s.commits);
  const setCommits = useSession((s) => s.setCommits);
  const mrs = useSession((s) => s.mrs);
  const mrThreadsByRepo = useSession((s) => s.mrThreadsByRepo);
  const openTab = useSession((s) => s.openTab);
  const resolveAnnotation = useSession((s) => s.resolveAnnotation);
  const removeAnnotation = useSession((s) => s.removeAnnotation);
  const isExplorer = useSession((s) => s.kind === 'explorer');
  const setLastError = useStore((s) => s.setLastError);
  const notify = useStore((s) => s.notify);

  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Breadcrumbs reveal a directory: expand it and every ancestor, and make sure
  // the tree is the visible panel.
  const revealDir = useStore((s) => s.revealDir);
  useEffect(() => {
    if (!revealDir?.path) return;
    const parts = revealDir.path.split('/').filter(Boolean);
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      parts.reduce((acc, part) => {
        const dir = acc ? `${acc}/${part}` : part;
        next.add(dir);
        return dir;
      }, '');
      return next;
    });
  }, [revealDir]);
  const setShowAddRepo = useStore((s) => s.setAddRepoOpen);
  // Git sub-mode lives in the session store so the global Alt+Shift+Tab shortcut
  // can cycle it; the buttons below write the same field.
  const gitSubTab = useSession((s) => s.gitSubTab);
  const setGitSubTab = useSession((s) => s.setGitSubTab);
  const commitLimit = useSession((s) => s.commitLimit);
  const commitsHasMore = useSession((s) => s.commitsHasMore);
  const loadMoreCommits = useSession((s) => s.loadMoreCommits);

  const worktreeForRepo = useCallback(
    (repoId: string) => activeWorktrees.find((w) => w.repo_id === repoId),
    [activeWorktrees]
  );

  // Opening the Forge sub-tab refreshes MRs + threads, so it never shows a
  // stale view of remote state (mr.* ops also refresh via useIpc).
  useEffect(() => {
    if (sidebarTab === 'git' && gitSubTab === 'forge') bumpMrs();
  }, [sidebarTab, gitSubTab, bumpMrs]);

  // Commit history follows the active repo (falls back to all repos if it has
  // no worktree). Reloads when the active repo changes, and when the list asks
  // for another page — one `git log` with a bigger limit, so pages never overlap
  // or interleave.
  useEffect(() => {
    if (!activeTask || sidebarTab !== 'git') return;
    const wt = activeWorktrees.find((w) => w.repo_id === activeRepoId);
    let cancelled = false;
    invoke<CommitEntry[]>('get_commit_log', { taskId: activeTask.short_id, worktreeId: wt?.id, limit: commitLimit })
      .then((c) => { if (!cancelled) setCommits(c, c.length >= commitLimit); })
      .catch((e) => { if (!cancelled) setLastError(String(e)); });
    return () => { cancelled = true; };
  }, [sidebarTab, activeTask, activeRepoId, activeWorktrees, commitLimit, setCommits, setLastError]);

  // Files and Source control focus their own keyboard list; Annotations has none,
  // so without this the panel shortcut could never tell it was already focused and
  // would never close. Focus the column itself as a fallback.
  const panelFocusNonce = useStore((s) => s.panelFocusNonce);
  const rootRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!panelFocusNonce) return;
    const t = window.setTimeout(() => {
      const root = rootRef.current;
      if (root && !root.contains(document.activeElement)) root.focus();
    }, 0);
    return () => window.clearTimeout(t);
  }, [panelFocusNonce]);

  if (!activeTask) return null;

  const activeRepo = activeRepos.find((r) => r.id === activeRepoId) ?? null;
  const activeWt = activeRepoId ? worktreeForRepo(activeRepoId) : undefined;
  const activeStatus = activeWt ? worktreeStatus[activeWt.id] : undefined;

  const openFileInEditor = (path: string, repoId: string, _lang?: string) => {
    openTab({ repoId, filePath: path, view: 'edit' });
  };

  const openFileInDiff = (path: string, repoId: string) => {
    openTab({ repoId, filePath: path, view: 'diff' });
  };

  const closeRepo = async (worktreeId: string) => {
    // force:true so closing still succeeds on a dirty worktree (the user has
    // already confirmed removal via the repo-switcher's confirm step).
    try {
      await invoke('close_worktree', { worktreeId, force: true });
    } catch (e) {
      setLastError(String(e));
    }
  };

  const makeGitAction = (worktreeId: string) => async (cmd: string) => {
    try {
      await invoke(cmd, { worktreeId });
      setTimeout(refreshStatus, 1500);
    } catch (e) {
      setLastError(String(e));
    }
  };

  const makeCommit = (worktreeId: string) => async (message: string) => {
    if (!message.trim()) return;
    try {
      await invoke('commit', { worktreeId, message: message.trim() });
      setTimeout(refreshStatus, 1500);
    } catch (e) {
      setLastError(String(e));
      throw e;
    }
  };

  // Staging: toggle one file, or stage/unstage the whole active repo. Refresh the
  // status counts + re-fetch the diff so the staged flags update.
  const refreshAfterStage = () => { refreshStatus(); bumpDiff(); };
  const toggleStage = async (path: string, repoId: string, staged: boolean) => {
    const wt = worktreeForRepo(repoId);
    if (!wt) return;
    try {
      await invoke(staged ? 'stage_file' : 'unstage_file', { worktreeId: wt.id, filePath: path });
      refreshAfterStage();
    } catch (e) {
      setLastError(String(e));
    }
  };
  const stageAll = (cmd: 'stage_all' | 'unstage_all') => async () => {
    if (!activeWt) return;
    try {
      await invoke(cmd, { worktreeId: activeWt.id });
      refreshAfterStage();
    } catch (e) {
      setLastError(String(e));
    }
  };

  // Discard is destructive → routed through the confirmation bridge (modal). The
  // confirmation_resolved handler refetches the diff + status after approval.
  const discardFile = (path: string, repoId: string) => {
    const wt = worktreeForRepo(repoId);
    if (!wt) return;
    invoke('discard_file', { worktreeId: wt.id, filePath: path }).catch((e) => setLastError(String(e)));
  };
  const discardAll = () => {
    if (!activeWt) return;
    invoke('discard_all', { worktreeId: activeWt.id }).catch((e) => setLastError(String(e)));
  };

  // Git sub-tab badges, scoped to the active repo (the chips show every repo).
  const activeDirty = activeStatus ? activeStatus.modified + activeStatus.staged : 0;
  const activeUnresolved = countUnresolved(mrThreadsByRepo[activeRepoId ?? ''] ?? []);

  const renderContent = () => {
    if (!activeRepo) {
      return <div className="sidebar-empty">No repositories in this task.<br />Add one to begin.</div>;
    }
    const repoId = activeRepo.id;

    if (sidebarTab === 'files') {
      return (
        <FilesTab
          repoId={repoId}
          worktreeForRepo={worktreeForRepo}
          expandedDirs={expandedDirs}
          onToggleDir={(path) =>
            setExpandedDirs((prev) => {
              const next = new Set(prev);
              if (next.has(path)) next.delete(path); else next.add(path);
              return next;
            })
          }
          onOpenFile={openFileInEditor}
        />
      );
    }

    if (sidebarTab === 'annotations') {
      const wt = worktreeForRepo(repoId);
      const repoMr = mrs.find((m) => m.worktree_id === wt?.id) ?? null;
      return (
        <AnnotationsTab
          annotations={annotations.filter((a) => a.repo_id === repoId)}
          repoFor={(id) => activeRepos.find((r) => r.id === id)}
          onResolve={async (id) => {
            try {
              await invoke('resolve_annotation', { id });
              resolveAnnotation(id);
            } catch (e) {
              setLastError(String(e));
            }
          }}
          onDelete={async (id) => {
            try {
              await invoke('delete_annotation', { id });
              removeAnnotation(id);
            } catch (e) {
              setLastError(String(e));
            }
          }}
          // Open the file at the annotated line (the editor takes cursorLine).
          onOpen={(a) =>
            openTab({
              repoId: a.repo_id,
              filePath: a.file_path,
              view: 'edit',
              cursorLine: a.start_line,
            })
          }
          mr={repoMr}
          onPostToMr={async (a) => {
            try {
              await invoke('post_mr_comment', {
                mrId: repoMr!.id,
                body: a.content,
                filePath: a.file_path,
                line: a.start_line,
              });
              await invoke('resolve_annotation', { id: a.id });
              resolveAnnotation(a.id);
              bumpMrs();
              notify({ kind: 'success', source: 'mr', taskId: a.session_id, title: `Comment posted on ${a.file_path.split('/').pop()}:${a.start_line}` });
            } catch (e) {
              setLastError(String(e));
            }
          }}
        />
      );
    }

    // git
    return (
      <div className="git-tab">
        <div className="git-subtabs">
          {(isExplorer ? (['changes', 'commits'] as const) : (['changes', 'commits', 'forge'] as const)).map((sub) => {
            const badge = sub === 'changes' ? activeDirty : sub === 'forge' ? activeUnresolved : 0;
            return (
              <button
                key={sub}
                className={`git-subtab ${gitSubTab === sub ? 'active' : ''}`}
                onClick={() => setGitSubTab(sub)}
              >
                {sub === 'changes' && 'Changes'}
                {sub === 'commits' && 'Commits'}
                {sub === 'forge' && 'Forge'}
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
          {gitSubTab === 'changes' && (() => {
            const repoFiles = diff?.repos.find((r) => r.repo_id === repoId)?.files ?? [];
            const stageable = repoFiles.filter((f) => f.staged != null);
            const anyUnstaged = stageable.some((f) => f.staged === false);
            return (
              <>
                {/* Diff base + refresh (moved here from the old modebar): this list
                    and every diff tab follow the selected base. */}
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
                        <span>{m.label}</span>
                      </button>
                    ))}
                  </div>
                  <button
                    className="diff-mode-refresh"
                    onClick={() => { bumpDiff(); refreshStatus(); }}
                    title="Refresh diff & git status"
                  >
                    <RefreshCw size={12} strokeWidth={1.75} />
                  </button>
                </div>
                <div className="git-changes-toolbar">
                  {stageable.length > 0 && (
                    <>
                      {anyUnstaged ? (
                        <button className="git-stage-all" onClick={stageAll('stage_all')} title="Stage all changes">
                          Stage all
                        </button>
                      ) : (
                        <button className="git-stage-all" onClick={stageAll('unstage_all')} title="Unstage all changes">
                          Unstage all
                        </button>
                      )}
                      <button className="git-discard-all" onClick={discardAll} title="Discard all local changes">
                        <RotateCcw size={13} strokeWidth={1.75} />
                      </button>
                    </>
                  )}
                </div>
                <ChangedFilesList
                  repoId={repoId}
                  onOpenFile={(path, rid) => openFileInDiff(path, rid)}
                  onOpenFileAlt={openFileInEditor}
                  onOpenAll={(rid) => openTab({ repoId: rid, filePath: '', view: 'diff', kind: 'changes' })}
                  onToggleStage={toggleStage}
                  onDiscard={discardFile}
                />
              </>
            );
          })()}
          {gitSubTab === 'forge' && !isExplorer && (() => {
            // All of the task's MRs across repos — selecting one opens its overview tab.
            const items = mrs.flatMap((m) => {
              const wt = activeWorktrees.find((w) => w.id === m.worktree_id);
              if (!wt) return [];
              const repo = activeRepos.find((r) => r.id === wt.repo_id);
              return [{
                mr: m,
                repoId: wt.repo_id,
                repoName: repo?.project ?? wt.repo_id,
                unresolved: countUnresolved(mrThreadsByRepo[wt.repo_id] ?? []),
              }];
            });
            return (
              <ForgeSection
                items={items}
                onSelect={(item) =>
                  openTab({
                    repoId: item.repoId,
                    filePath: '',
                    view: 'diff',
                    kind: 'mr',
                    mrId: item.mr.id,
                    label: `${item.mr.platform === 'github' ? '#' : '!'}${item.mr.remote_id}`,
                  })
                }
              />
            );
          })()}
        </div>
      </div>
    );
  };

  return (
    <aside className="sidebar" ref={rootRef} tabIndex={-1}>

      <RepoSwitcher
        repos={activeRepos}
        activeRepoId={activeRepoId}
        worktreeForRepo={worktreeForRepo}
        worktreeStatus={worktreeStatus}
        onSelect={setActiveRepoId}
        onAddRepo={() => setShowAddRepo(true)}
        onCloseRepo={closeRepo}
      />

      <div className="sidebar-content">{renderContent()}</div>

      {/* Docked footer for the active repo: the commit composer, whatever tab is
          showing. A "remote branch deleted" banner used to pre-empt it, but a
          deleted remote branch is the NORMAL state after a merge — the name is free
          to reuse, and git already refuses a second worktree on one branch. */}
      {activeWt && (
        <>
          <GitCommitPanel
            key={activeWt.id}
            status={activeStatus}
            branch={activeWt.branch}
            worktreeId={activeWt.id}
            commitOnly={isExplorer}
            hasMr={mrs.some((m) => m.worktree_id === activeWt.id)}
            onCommit={makeCommit(activeWt.id)}
            onAction={makeGitAction(activeWt.id)}
          />
        </>
      )}
    </aside>
  );
}
