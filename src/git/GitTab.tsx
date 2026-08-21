import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import {
  GitCommit, Upload, Download, ChevronsUp, ChevronDown, Plus, Minus, Circle, Trash2, AlertTriangle,
  GitPullRequest, GitCompare, Search, X,
} from 'lucide-react';
import { useSession, useStore } from '../shared/store';
import { useListNav } from '../shared/lib/useListNav';
import type { CommitEntry, WorktreeStatus } from '../shared/ipc/ipc';
import { guessLang } from '../shared/lib/lang';
import { StatBadge } from '../shared/ui/StatBadge';
import { registerCommitPush } from '../shared/lib/gitChain';
import { Highlighted, matchRanges } from '../shared/lib/match';

/** Git status indicator: green + (added), yellow dot (modified), red − (deleted). */
function FileStatusIcon({ status }: { status: string }) {
  const st = status === 'A' || status === 'D' ? status : 'M';
  return (
    <span className={`changed-file-status st-${st}`}>
      {st === 'A' && <Plus size={13} strokeWidth={2.5} />}
      {st === 'D' && <Minus size={13} strokeWidth={2.5} />}
      {st === 'M' && <Circle size={7} strokeWidth={0} fill="currentColor" />}
    </span>
  );
}

// ── Commits tab ───────────────────────────────────────────────────────────────

/**
 * The commit log: a fuzzy filter over what is loaded, and another page whenever
 * the list is scrolled to the end.
 *
 * The filter is local rather than a `git log --grep`: it matches the message, the
 * author and the sha in one pass, tolerates gaps ("fxauth" finds "fix: auth"), and
 * answers instantly. What it cannot see is a commit that has not been fetched yet,
 * so the footer always says how many are loaded and offers the next page.
 */
export function CommitsTab({
  commits, hasMore, onLoadMore, onSelect,
}: {
  commits: CommitEntry[];
  hasMore: boolean;
  onLoadMore: () => void;
  onSelect: (c: CommitEntry) => void;
}) {
  const [query, setQuery] = useState('');
  const endRef = useRef<HTMLDivElement | null>(null);

  const matches = useMemo(() => {
    if (!query.trim()) return commits.map((c) => ({ c, ranges: [] as [number, number][] | null }));
    return commits
      .map((c) => {
        // One field decides the highlight (the message, where the eye goes), but a
        // hit on the author or the sha still keeps the row.
        const ranges = matchRanges(query, c.message);
        const other = matchRanges(query, c.author) ?? matchRanges(query, c.short_sha);
        return ranges || other ? { c, ranges } : null;
      })
      .filter((x): x is { c: CommitEntry; ranges: [number, number][] | null } => x !== null);
  }, [commits, query]);

  // Reaching the end of the list IS the request for more. The observer is rebuilt
  // when the page grows so it re-arms against the new last row.
  useEffect(() => {
    const el = endRef.current;
    if (!el || !hasMore) return;
    const io = new IntersectionObserver(
      (entries) => { if (entries.some((e) => e.isIntersecting)) onLoadMore(); },
      { threshold: 0.1 },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [hasMore, onLoadMore, commits.length]);

  // The divergence point: everything from the first base commit down is upstream
  // history the branch grew from, rendered dimmed under a divider. Filtering hides
  // the boundary, so the divider is only drawn on the unfiltered list.
  const filtering = query.trim().length > 0;
  const firstBaseSha = filtering ? null : commits.find((c) => c.is_base)?.sha;

  return (
    <div className="commits-tab">
      <div className="commits-search">
        <Search size={12} strokeWidth={2} className="commits-search-icon" />
        <input
          className="commits-search-input"
          placeholder="Filter commits…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') { e.stopPropagation(); setQuery(''); }
          }}
        />
        {query && (
          <button className="commits-search-clear" onClick={() => setQuery('')} title="Clear">
            <X size={11} strokeWidth={2.25} />
          </button>
        )}
      </div>

      {commits.length === 0 ? (
        <div className="sidebar-empty">No commits</div>
      ) : matches.length === 0 ? (
        <div className="sidebar-empty">
          No match in the {commits.length} commits loaded.
          {hasMore && <button className="home-link" onClick={onLoadMore}>load more</button>}
        </div>
      ) : (
        <div className="commits-list">
          {matches.map(({ c, ranges }) => (
            <div key={c.sha}>
              {c.sha === firstBaseSha && commits[0]?.sha !== c.sha && (
                <div className="commit-base-divider">
                  <span>base</span>
                </div>
              )}
              <button
                className={`commit-item ${c.is_base ? 'base' : ''}`}
                onClick={() => onSelect(c)}
                title={`${c.message} — view this commit's changes`}
              >
                <span className="commit-msg"><Highlighted text={c.message} ranges={ranges} /></span>
                <span className="commit-meta">
                  <GitCommit size={11} strokeWidth={1.75} className="commit-icon" />
                  <span className="commit-sha">{c.short_sha}</span>
                  <span className="commit-author">{c.author}</span>
                </span>
              </button>
            </div>
          ))}
          {/* The sentinel: visible = the user reached the end. */}
          <div ref={endRef} className="commits-end">
            {hasMore ? `${commits.length} loaded — more…` : `${commits.length} commits`}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Changed files (flat list) ──────────────────────────────────────────────────

export function ChangedFilesList({
  repoId, onOpenFile, onOpenFileAlt, onOpenAll, onToggleStage, onDiscard, onStageAll, onDiscardAll,
}: {
  repoId: string | null;
  onOpenFile: (path: string, repoId: string, lang: string) => void;
  onOpenFileAlt: (path: string, repoId: string, lang: string) => void;
  /** Open the whole repo's changes as one review tab. */
  onOpenAll: (repoId: string) => void;
  onToggleStage: (path: string, repoId: string, staged: boolean) => void;
  onDiscard: (path: string, repoId: string) => void;
  /** Stage or unstage every file at once (the All-changes row's checkbox). */
  onStageAll: (stage: boolean) => void;
  /** Discard every local change (the All-changes row's trash). */
  onDiscardAll: () => void;
}) {
  const diff = useSession((s) => s.diff);
  const panelFocusNonce = useStore((s) => s.panelFocusNonce);
  const files = repoId ? (diff?.repos.find((r) => r.repo_id === repoId)?.files ?? []) : [];

  // "All changes" is row 0, so it is reachable by keyboard like any file. Every
  // nav index is therefore one ahead of its file: index 1 is files[0].
  const totals = files.reduce(
    (acc, f) => ({ add: acc.add + f.added, del: acc.del + f.deleted }),
    { add: 0, del: 0 },
  );
  const stageable = files.filter((f) => f.staged != null);
  const anyUnstaged = stageable.some((f) => f.staged === false);
  const openAll = useCallback(() => { if (repoId) onOpenAll(repoId); }, [repoId, onOpenAll]);

  // Enter stages/unstages — the action you repeat while reviewing your own work.
  // Opening moves to l / ArrowRight (and click), which is what the file tree uses
  // for "go into this". On row 0 both mean "open the review".
  const onEnter = useCallback((i: number) => {
    if (i === 0) return openAll();
    const f = files[i - 1];
    if (f && repoId && f.staged != null) onToggleStage(f.path, repoId, !(f.staged === true));
  }, [files, repoId, onToggleStage, openAll]);
  const onRight = useCallback((i: number) => {
    if (i === 0) return openAll();
    const f = files[i - 1];
    if (f && repoId) onOpenFile(f.path, repoId, guessLang(f.path));
  }, [files, repoId, onOpenFile, openAll]);
  const nav = useListNav({ count: files.length + 1, onEnter, onRight, focusNonce: panelFocusNonce });

  if (!repoId) return <div className="sidebar-empty">Select a repo above</div>;
  if (files.length === 0) return <div className="sidebar-empty">No changed files</div>;

  return (
    <div className="files-list nav-list" tabIndex={0} ref={nav.containerRef} onKeyDown={nav.onKeyDown}>
      <div className="changed-file-row">
        <button
          className={`changed-file changed-file-all ${nav.index === 0 ? 'nav-selected' : ''}`}
          title="Open all of this repo's changes in one review tab"
          tabIndex={-1}
          onClick={() => { nav.setIndex(0); openAll(); }}
        >
          <GitCompare size={12} strokeWidth={1.75} className="changed-file-all-icon" />
          <span className="changed-file-name">All changes</span>
          <span className="changed-file-dir">{files.length} file{files.length === 1 ? '' : 's'}</span>
          <StatBadge stat={totals} />
        </button>
        {stageable.length > 0 && (
          <>
            <input
              type="checkbox"
              className="changed-file-checkbox"
              checked={!anyUnstaged}
              title={anyUnstaged ? 'Stage all changes' : 'Unstage all changes'}
              onChange={() => onStageAll(anyUnstaged)}
            />
            <button
              className="changed-file-discard"
              title="Discard all local changes"
              onClick={(e) => { e.stopPropagation(); onDiscardAll(); }}
            >
              <Trash2 size={12} strokeWidth={1.75} />
            </button>
          </>
        )}
      </div>
      {files.map((f, fi) => {
        const i = fi + 1;
        const name = f.path.split('/').pop() ?? f.path;
        const dir = f.path.slice(0, Math.max(0, f.path.length - name.length - 1));
        return (
          <div key={f.path} className="changed-file-row">
            <button
              className={`changed-file ${i === nav.index ? 'nav-selected' : ''}`}
              title={f.path}
              tabIndex={-1}
              onClick={() => { nav.setIndex(i); onOpenFile(f.path, repoId, guessLang(f.path)); }}
              onDoubleClick={() => onOpenFileAlt(f.path, repoId, guessLang(f.path))}
            >
              <FileStatusIcon status={f.status} />
              <span className="changed-file-name">{name}</span>
              {dir && <span className="changed-file-dir">{dir}</span>}
              <StatBadge stat={{ add: f.added, del: f.deleted }} />
            </button>
            {f.staged != null && (
              <>
                <input
                  type="checkbox"
                  className="changed-file-checkbox"
                  checked={f.staged === true}
                  title={f.staged ? 'Staged — click to unstage' : 'Stage this file'}
                  onChange={() => onToggleStage(f.path, repoId, !(f.staged === true))}
                />
                <button
                  className="changed-file-discard"
                  title="Discard changes to this file"
                  onClick={(e) => { e.stopPropagation(); onDiscard(f.path, repoId); }}
                >
                  <Trash2 size={12} strokeWidth={1.75} />
                </button>
              </>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ── Git commit panel ──────────────────────────────────────────────────────────

// Each menu entry maps to a single primary action. `commit`/`commit-push` need a
// message; the rest act on the branch. Backend commands: commit · push · pull ·
// rebase_on_main (there is no dedicated pull-with-rebase).
type ActionKey = 'commit' | 'commit-push' | 'push' | 'pull' | 'rebase' | 'create-mr';

const GIT_MENU: { key: ActionKey; label: string; icon: typeof GitCommit; needsMessage?: boolean }[] = [
  { key: 'commit',      label: 'Commit',         icon: GitCommit, needsMessage: true },
  { key: 'commit-push', label: 'Commit & Push',  icon: Upload,    needsMessage: true },
  { key: 'push',        label: 'Push',           icon: Upload },
  { key: 'pull',        label: 'Pull',           icon: Download },
  { key: 'rebase',      label: 'Rebase on main', icon: ChevronsUp },
  { key: 'create-mr',   label: 'Create MR…',     icon: GitPullRequest },
];

/**
 * Docked commit composer for the active repo. A single context-aware primary
 * button (Commit → Push → Pull, derived from message + git status) with a ▾
 * menu for every action, plus inline status chips.
 */
export function GitCommitPanel({
  status, branch, worktreeId, commitOnly = false, hasMr = false, onCommit, onAction,
}: {
  status?: WorktreeStatus;
  branch?: string;
  /** The active worktree — scopes the rebase-conflict banner to this repo. */
  worktreeId?: string;
  /** Explorer sessions: only commit is available (no push/pull/rebase). */
  commitOnly?: boolean;
  /** This branch already has an MR — hides "Create MR…". */
  hasMr?: boolean;
  /** Resolves to the commit confirmation's id (chains Commit & Push). */
  onCommit: (message: string) => Promise<string | undefined | void>;
  onAction: (cmd: string) => void;
}) {
  const [message, setMessage] = useState('');
  const [menuOpen, setMenuOpen] = useState(false);
  const [committing, setCommitting] = useState(false);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const menu = commitOnly
    ? GIT_MENU.filter((a) => a.key === 'commit')
    : GIT_MENU.filter((a) => a.key !== 'create-mr' || !hasMr);

  // ── Rebase-conflict flow + commit focus ─────────────────────────────────────
  const rebaseConflict = useSession((s) => s.rebaseConflict);
  const commitFocusNonce = useStore((s) => s.commitFocusNonce);
  const [rebaseError, setRebaseError] = useState<string | null>(null);
  const initialFocusNonce = useRef(commitFocusNonce);

  // Focus the commit textarea when a commit is requested (e.g. from the command
  // palette). Skip the value present on the initial mount.
  useEffect(() => {
    if (commitFocusNonce === initialFocusNonce.current) return;
    taRef.current?.focus();
  }, [commitFocusNonce]);

  // Only surface the banner for the worktree that actually hit the conflict.
  const conflict = rebaseConflict && (!worktreeId || rebaseConflict.worktreeId === worktreeId)
    ? rebaseConflict : null;

  const runRebaseAction = async (cmd: 'rebase_continue' | 'rebase_abort') => {
    if (!conflict) return;
    setRebaseError(null);
    try {
      await invoke(cmd, { worktreeId: conflict.worktreeId });
    } catch (e) {
      setRebaseError(String(e));
    }
  };

  // Auto-grow the textarea between ~1.5 and ~7 lines.
  const autosize = () => {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(ta.scrollHeight, 150)}px`;
  };
  useEffect(autosize, [message]);

  const hasMsg = message.trim().length > 0;
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;
  const dirty = (status?.modified ?? 0) + (status?.staged ?? 0);

  // The smart default that the big button performs.
  const primary: ActionKey | null = commitOnly
    ? (hasMsg ? 'commit' : null)
    : hasMsg ? 'commit' : behind > 0 ? 'pull' : ahead > 0 ? 'push' : null;

  const run = async (key: ActionKey) => {
    setMenuOpen(false);
    if (key === 'commit' || key === 'commit-push') {
      if (!hasMsg || committing) return;
      setCommitting(true);
      try {
        const confirmationId = await onCommit(message.trim());
        setMessage('');
        // The push posts only after the commit RESOLVES approved (useIpc) —
        // posting both at once could push a tree whose commit was denied.
        if (key === 'commit-push' && typeof confirmationId === 'string' && worktreeId) {
          registerCommitPush(confirmationId, worktreeId);
        }
      } catch { /* keep the message so it isn't lost */ }
      finally { setCommitting(false); }
    } else if (key === 'push') onAction('push');
    else if (key === 'pull') onAction('pull');
    else if (key === 'rebase') onAction('rebase_on_main');
    // Opens the confirmation dialog; the title/description are typed there.
    else if (key === 'create-mr') onAction('create_mr');
  };

  const primaryEntry = primary ? GIT_MENU.find((a) => a.key === primary)! : null;
  const PrimaryIcon = primaryEntry?.icon ?? GitCommit;

  return (
    <div className="git-commit-panel">
      {conflict && (
        <div className="git-warn-banner sidebar-footer-banner">
          <div className="git-warn-header">
            <AlertTriangle size={12} strokeWidth={2} style={{ color: 'var(--wb-warn)', flexShrink: 0 }} />
            <span>Rebase conflict — {conflict.files.length} file{conflict.files.length === 1 ? '' : 's'}</span>
          </div>
          <div className="git-warn-body">
            {conflict.files.map((f) => <div key={f}>{f}</div>)}
          </div>
          {rebaseError && <div className="mr-thread-resolve-error">{rebaseError}</div>}
          <button className="git-warn-close-btn" onClick={() => runRebaseAction('rebase_continue')}>
            Continue rebase
          </button>
          <button className="git-warn-close-btn" onClick={() => runRebaseAction('rebase_abort')}>
            Abort rebase
          </button>
        </div>
      )}
      <textarea
        ref={taRef}
        className="git-commit-textarea"
        placeholder={branch ? `Message — commit to ${branch} (Ctrl+Enter)` : 'Message (Ctrl+Enter to commit)'}
        rows={3}
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyDown={(e) => { if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') run('commit'); }}
      />
      <div className="git-commit-actions">
        <span className="git-commit-stats">
          {dirty > 0 && status!.modified > 0 && <span className="repo-stat-modified" title={`${status!.modified} modified`}>~{status!.modified}</span>}
          {dirty > 0 && status!.staged > 0 && <span className="repo-stat-staged" title={`${status!.staged} staged`}>+{status!.staged}</span>}
          {!commitOnly && ahead > 0 && <span className="repo-stat-ahead" title={`${ahead} ahead`}>↑{ahead}</span>}
          {!commitOnly && behind > 0 && <span className="repo-stat-behind" title={`${behind} behind`}>↓{behind}</span>}
          {dirty === 0 && (commitOnly || (ahead === 0 && behind === 0)) && status && <span className="git-commit-clean">{commitOnly ? 'No changes' : 'Up to date'}</span>}
        </span>

        <div className="git-split">
          <button
            className="git-split-main"
            disabled={!primary || committing}
            onClick={() => primary && run(primary)}
            title={primaryEntry?.label ?? 'Nothing to do'}
          >
            <PrimaryIcon size={12} strokeWidth={1.75} />
            <span>{committing ? 'Working…' : primaryEntry?.label ?? 'Commit'}</span>
          </button>
          {menu.length > 1 && (
            <button className="git-split-toggle" onClick={() => setMenuOpen((o) => !o)} aria-label="Choose git action">
              <ChevronDown size={12} strokeWidth={2} />
            </button>
          )}
          {menuOpen && (
            <>
              <div className="git-split-backdrop" onClick={() => setMenuOpen(false)} />
              <div className="git-split-menu">
                {menu.map((a) => {
                  const AIcon = a.icon;
                  const disabled = a.needsMessage && !hasMsg;
                  return (
                    <button
                      key={a.key}
                      className={`git-split-item ${a.key === primary ? 'active' : ''}`}
                      disabled={disabled}
                      onClick={() => run(a.key)}
                    >
                      <AIcon size={12} strokeWidth={1.75} />
                      <span>{a.label}</span>
                    </button>
                  );
                })}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
