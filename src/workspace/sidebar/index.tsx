import { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '../../shared/ipc/invoke';
import { useStore, useSession } from '../../shared/store';
import { activeWorktreeFor } from '../../shared/lib/workspace';
import type { CommitEntry } from '../../shared/ipc/ipc';
import { FilesTab } from '../../files/FilesTab';
import { GitCommitPanel } from '../../git/GitCommitPanel';
import { NotesPanel } from './NotesPanel';
import { GitPanel } from './GitPanel';

export function Sidebar() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const sidebarTab = useSession((s) => s.sidebarTab);
  const activeRepoId = useSession((s) => s.activeRepoId);
  const worktreeStatus = useSession((s) => s.worktreeStatus);
  const bumpMrs = useSession((s) => s.bumpMrs);
  const commitLimit = useSession((s) => s.commitLimit);
  const setCommits = useSession((s) => s.setCommits);
  const mrs = useSession((s) => s.mrs);
  const openTab = useSession((s) => s.openTab);
  const isExplorer = useSession((s) => s.kind === 'explorer');
  const setLastError = useStore((s) => s.setLastError);

  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Reveals a directory: expands it and every ancestor, and shows the tree panel.
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

  const activeWorktreeId = useSession((s) => s.activeWorktreeId);
  const worktreeForRepo = useCallback(
    (repoId: string) => activeWorktreeFor(activeWorktrees, repoId, activeWorktreeId),
    [activeWorktrees, activeWorktreeId]
  );

  // Opening the git tab refreshes the MRs and their threads.
  useEffect(() => {
    if (sidebarTab === 'git') bumpMrs();
  }, [sidebarTab, bumpMrs]);

  // Commit history follows the active worktree, all repos when it has none.
  // One `git log` with a bigger limit per page: pages never overlap.
  useEffect(() => {
    if (!activeTask || sidebarTab !== 'git') return;
    const wt = activeRepoId ? worktreeForRepo(activeRepoId) : undefined;
    let cancelled = false;
    invoke<CommitEntry[]>('get_commit_log', { taskId: activeTask.short_id, worktreeId: wt?.id, limit: commitLimit })
      .then((c) => { if (!cancelled) setCommits(c, c.length >= commitLimit); })
      .catch((e) => { if (!cancelled) setLastError(e); });
    return () => { cancelled = true; };
  }, [sidebarTab, activeTask, activeRepoId, worktreeForRepo, commitLimit, setCommits, setLastError]);

  // Annotations has no keyboard list: focus the column itself, or the panel shortcut cannot close it.
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

  const makeGitAction = (worktreeId: string) => async (cmd: string) => {
    try {
      await invoke(cmd, { worktreeId });
    } catch (e) {
      setLastError(e);
    }
  };

  const makeCommit = (worktreeId: string) => async (message: string) => {
    if (!message.trim()) return undefined;
    try {
      // Confirmation id: Commit & Push chains off it.
      return await invoke<string>('commit', { worktreeId, message: message.trim() });
    } catch (e) {
      setLastError(e);
      throw e;
    }
  };

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
      return <NotesPanel repoId={repoId} worktreeForRepo={worktreeForRepo} />;
    }

    return (
      <GitPanel
        repoId={repoId}
        activeWt={activeWt}
        // Scoped to the active repo (the chips show every repo).
        activeDirty={activeStatus ? activeStatus.modified + activeStatus.staged : 0}
        worktreeForRepo={worktreeForRepo}
        onOpenFileInEditor={openFileInEditor}
      />
    );
  };

  return (
    <aside className="sidebar" ref={rootRef} tabIndex={-1}>

      <div className="sidebar-content">{renderContent()}</div>

      {/* Docked footer for the active repo: the commit composer, on whichever tab shows. */}
      {activeWt && (
        <GitCommitPanel
          key={activeWt.id}
          status={activeStatus}
          branch={activeWt.branch}
          worktreeId={activeWt.id}
          commitOnly={isExplorer}
          mr={mrs.find((m) => m.worktree_id === activeWt.id)}
          onCommit={makeCommit(activeWt.id)}
          onAction={makeGitAction(activeWt.id)}
        />
      )}
    </aside>
  );
}
