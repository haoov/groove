import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useStore, useSession } from '../shared/store';
import type { Annotation, Hunk, RepoDiff, Mr, MrThread } from '../shared/ipc/ipc';
import { mrForWorktree } from '../shared/lib/workspace';
import { FileDiffEditor } from '../editor/FileDiffEditor';
import { useDiffExpand } from '../editor/useDiffExpand';
import { useBlame } from '../editor/useBlame';
import type { AnnCtx } from '../editor/useAnnotations';

/**
 * "All changes" tab content: one repo's changed files, stacked and individually
 * expandable — the scroll-through review surface. Hunks load lazily per file.
 */
export function ChangesView({ repoId, ann }: { repoId: string; ann: AnnCtx }) {
  const diff = useSession((s) => s.diff);
  const diffHunks = useSession((s) => s.diffHunks);
  const diffMode = useSession((s) => s.diffMode);
  const setDiffHunks = useSession((s) => s.setDiffHunks);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const annotations = useSession((s) => s.annotations);
  const mrThreadsByRepo = useSession((s) => s.mrThreadsByRepo);
  const mrs = useSession((s) => s.mrs);
  const expandedFiles = useSession((s) => s.expandedDiffFiles);
  const toggleFile = useSession((s) => s.toggleDiffFile);
  const setLastError = useStore((s) => s.setLastError);

  const hunksInFlight = useRef<Set<string>>(new Set());

  const repo = diff?.repos.find((r) => r.repo_id === repoId);
  const wt = activeWorktrees.find((w) => w.repo_id === repoId);

  // Lazily fetch line content for expanded files not yet cached.
  useEffect(() => {
    if (!repo || !wt) return;
    for (const f of repo.files) {
      const key = `${repoId}/${f.path}`;
      if (!expandedFiles.has(key) || diffHunks[key] !== undefined || hunksInFlight.current.has(key)) continue;
      hunksInFlight.current.add(key);
      invoke<Hunk[]>('get_file_diff', { worktreeId: wt.id, filePath: f.path, mode: diffMode })
        .then((hunks) => setDiffHunks(key, hunks))
        .catch((e) => setLastError(String(e)))
        .finally(() => hunksInFlight.current.delete(key));
    }
  }, [expandedFiles, repo, wt, diffHunks, diffMode, repoId, setDiffHunks, setLastError]);

  if (!repo || repo.files.length === 0) {
    return <div className="diff-empty"><p>No changes in this repo</p></div>;
  }

  return (
    <div className="diff-view changes-view" onClick={() => ann.cancel()}>
      <RepoDiffSection
        repo={repo}
        worktreeId={wt?.id}
        expandedFiles={expandedFiles}
        onToggleFile={toggleFile}
        diffHunks={diffHunks}
        annotations={annotations}
        threads={mrThreadsByRepo[repoId] ?? []}
        mr={mrForWorktree(mrs, wt?.id)}
        ann={ann}
      />
    </div>
  );
}

/** One repo's changed files, stacked and individually expandable. */
export function RepoDiffSection({
  repo, worktreeId, expandedFiles, onToggleFile, diffHunks, annotations, threads, mr, ann,
}: {
  repo: RepoDiff;
  /** Needed to read more context around a hunk; omit to disable expansion. */
  worktreeId?: string;
  expandedFiles: Set<string>;
  onToggleFile: (key: string) => void;
  diffHunks: Record<string, Hunk[]>;
  annotations: Annotation[];
  threads: MrThread[];
  mr: Mr | null;
  ann: AnnCtx;
}) {
  const openAnns = annotations.filter((a) => a.repo_id === repo.repo_id && a.status === 'open');

  // File-level keyboard navigation. The editors inside expanded files own j/k for
  // lines (vim); this moves between the file HEADERS, which was otherwise
  // mouse-only. Keys are handled on the container and ignored while focus is
  // inside a CodeMirror instance, so the two never fight.
  const listRef = useRef<HTMLDivElement>(null);
  const [cursor, setCursor] = useState(0);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if ((e.target as HTMLElement).closest('.cm-editor')) return;
    const last = repo.files.length - 1;
    const move = (to: number) => {
      e.preventDefault();
      const at = Math.max(0, Math.min(last, to));
      setCursor(at);
      listRef.current
        ?.querySelectorAll('.diff-file-header')[at]
        ?.scrollIntoView({ block: 'nearest' });
    };
    switch (e.key) {
      case 'j': case 'ArrowDown': return move(cursor + 1);
      case 'k': case 'ArrowUp':   return move(cursor - 1);
      case 'g': case 'Home':      return move(0);
      case 'G': case 'End':       return move(last);
      case 'Enter': case ' ': {
        e.preventDefault();
        const file = repo.files[cursor];
        if (file) onToggleFile(`${repo.repo_id}/${file.path}`);
        return;
      }
    }
  };

  return (
    <div
      className="diff-repo"
      ref={listRef}
      tabIndex={-1}
      onKeyDown={onKeyDown}
      // Clicking a header hands the keyboard to the list, so j/k works without
      // a separate "focus the diff" step.
      onMouseDown={(e) => {
        if ((e.target as HTMLElement).closest('.cm-editor')) return;
        listRef.current?.focus();
      }}
    >
      {repo.files.map((file, i) => {
        const key = `${repo.repo_id}/${file.path}`;
        const expanded = expandedFiles.has(key);
        const fileAnns = openAnns.filter((a) => a.file_path === file.path);

        return (
          <div key={file.path} className="diff-file">
            <div
              className={`diff-file-header ${i === cursor ? 'cursor' : ''}`}
              onClick={() => { setCursor(i); onToggleFile(key); }}
            >
              <span className="diff-expand">{expanded ? '▾' : '▸'}</span>
              <span className="diff-file-path">{file.path}</span>
              {fileAnns.length > 0 && <span className="diff-annotation-badge">{fileAnns.length}</span>}
              <button
                className="diff-open-editor-btn"
                title="Open in editor"
                onClick={(e) => { e.stopPropagation(); ann.openInEditor(repo.repo_id, file.path); }}
              >↗</button>
            </div>

            {expanded && (
              <div className="diff-hunks">
                {diffHunks[key] === undefined ? (
                  <div className="diff-file-loading">Loading diff…</div>
                ) : diffHunks[key].length === 0 ? (
                  <div className="diff-file-loading">No text changes (binary file).</div>
                ) : (
                  <ExpandableFileDiff
                    hunks={diffHunks[key]}
                    worktreeId={worktreeId}
                    filePath={file.path}
                    repoId={repo.repo_id}
                    ann={ann}
                    fileAnnotations={fileAnns}
                    threads={threads}
                    mr={mr}
                  />
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/** One file's diff, wrapped so each file owns its own context expansion. */
function ExpandableFileDiff({
  hunks, worktreeId, filePath, repoId, ann, fileAnnotations, threads, mr,
}: {
  hunks: Hunk[];
  worktreeId?: string;
  filePath: string;
  repoId: string;
  ann: AnnCtx;
  fileAnnotations: Annotation[];
  threads: MrThread[];
  mr: Mr | null;
}) {
  const setDiffHunks = useSession((s) => s.setDiffHunks);
  const key = `${repoId}/${filePath}`;
  const expand = useDiffExpand({
    worktreeId, filePath, hunks,
    onHunks: (h) => setDiffHunks(key, h),
  });
  const { blameOn, blame, openCommit } = useBlame({ worktreeId, repoId, filePath });

  return (
    <FileDiffEditor
      hunks={hunks}
      filePath={filePath}
      repoId={repoId}
      ann={ann}
      sel={ann.sel}
      dragRange={ann.dragRange}
      fileAnnotations={fileAnnotations}
      threads={threads}
      mr={mr}
      onExpandGap={expand.onExpand}
      fileLineCount={expand.total}
      blame={blameOn ? blame : undefined}
      onOpenCommit={openCommit}
    />
  );
}
