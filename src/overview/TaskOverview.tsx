import { useEffect, useState } from 'react';
import { openExternal } from '../shared/lib/openExternal';
import { invoke } from '../shared/ipc/invoke';
import { providerCopy } from '../shared/lib/taskProvider';
import type { TaskSchema } from '../shared/ipc/ipc';
import { CheckCircle2, AlertTriangle, Trash2, X, RefreshCw, ExternalLink, Plus } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import type { Mr } from '../shared/ipc/ipc';
import { RepoRow } from './parts';
import { PropertyStrip } from './PropertyStrip';
import { BodyEditor } from './BodyEditor';

export function TaskOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const activeRepos = useSession((s) => s.activeRepos);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const setActiveWorktreeId = useSession((s) => s.setActiveWorktreeId);
  const setWorkspaceMode = useSession((s) => s.setWorkspaceMode);
  const setLastError = useStore((s) => s.setLastError);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);

  // Clicking a worktree scopes the editor to it and leaves the overview.
  const openWorktree = (repoId: string, worktreeId: string) => {
    setActiveRepoId(repoId);
    setActiveWorktreeId(worktreeId);
    setWorkspaceMode('code');
  };
  const [allMrs, setAllMrs] = useState<Mr[]>([]);
  const [body, setBody] = useState('');
  const [loading, setLoading] = useState(false);
  /** Which ending is awaiting confirmation. Finishing marks the task Done;
   *  deleting discards it at the source. Both then tear the local
   *  workspace down, so they share one banner and one busy flag. */
  const [ending, setEnding] = useState<'finish' | 'delete' | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const src = providerCopy(activeTask);
  const [reloadNonce, setReloadNonce] = useState(0);
  const reload = () => setReloadNonce((n) => n + 1);

  useEffect(() => {
    if (!activeTask) return;
    setLoading(true);
    // Markdown, not raw blocks: same renderer as MR descriptions, and markdown
    // typed literally into the source (backticks, **bold**) then displays properly.
    invoke<string>('get_task_body_markdown', { shortId: activeTask.short_id })
      .then(setBody)
      .catch(() => setBody(''))
      .finally(() => setLoading(false));
  }, [activeTask?.short_id, reloadNonce]);

  // Per task, not per mount: a schema belongs to the source the task came from.
  useEffect(() => {
    if (!activeTask) return;
    invoke<TaskSchema>('get_task_schema', { shortId: activeTask.short_id })
      .then(setSchema)
      .catch(() => setSchema(null));
  }, [activeTask?.short_id]);

  // Load MRs for all worktrees into local state — independent of the sidebar's
  // per-repo store so switching repos in the sidebar never clears this list.
  useEffect(() => {
    if (!activeWorktrees.length) return;
    let cancelled = false;
    Promise.all(
      activeWorktrees.map((wt) =>
        invoke<Mr[]>('get_mr', { worktreeId: wt.id }).catch(() => [] as Mr[])
      )
    ).then((results) => { if (!cancelled) setAllMrs(results.flat()); });
    return () => { cancelled = true; };
  }, [activeWorktrees]);

  // No success path to handle: both commands emit task_finished, which closes the
  // session — this component goes with it.
  const handleEnd = async () => {
    if (!activeTask || !ending) return;
    setFinishing(true);
    try {
      await invoke(ending === 'delete' ? 'delete_task' : 'finish_task', {
        shortId: activeTask.short_id,
      });
    } catch (e) {
      setLastError(String(e));
      setFinishing(false);
      setEnding(null);
    }
  };

  if (!activeTask) return null;

  return (
    <div className="overview-view">
      <div className="overview-inner">
        {/* Header — id chip, title, and the task-level actions. */}
        <header className="overview-header">
          <span className="overview-task-id">{activeTask.short_id}</span>
          <h1 className="overview-title">{activeTask.title}</h1>
          <span className="overview-spring" />
          <div className="overview-header-actions">
            {activeTask.external_url && (
              <button
                className="finish-task-btn ov-update"
                onClick={() => openExternal(activeTask.external_url!)}
                title={activeTask.external_url}
              >
                <ExternalLink size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
                Open in {src.label}
              </button>
            )}
            <button
              className="finish-task-btn ov-update"
              onClick={reload}
              title={`Re-read this task from ${src.label}`}
            >
              <RefreshCw size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
              Refresh
            </button>
            <button
              className="finish-task-btn"
              onClick={() => setEnding('finish')}
              disabled={finishing}
            >
              <CheckCircle2 size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
              Finish task
            </button>
            <button
              className="finish-task-btn delete-task-btn"
              onClick={() => setEnding('delete')}
              disabled={finishing}
              title={`Discard the ${src.item} and close the task here`}
            >
              <Trash2 size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
              Delete task
            </button>
          </div>
        </header>

        {/* Properties — the framed metadata card under the title. */}
        <PropertyStrip
          key={reloadNonce}
          shortId={activeTask.short_id}
          schema={schema}
        />

        {/* One banner for both endings — the local half is identical, so only the
            sentence about what changes at the source. */}
        {ending && (
          <div className={`finish-confirm-banner ${ending === 'delete' ? 'destructive' : ''}`}>
            <div className="finish-confirm-icon">
              <AlertTriangle size={14} strokeWidth={2} />
            </div>
            <div className="finish-confirm-body">
              <strong>
                {ending === 'delete' ? 'Delete' : 'Finish'} &ldquo;{activeTask.title}&rdquo;?
              </strong>
              <p>
                This will remove all local worktrees and delete task data from the local
                database.{' '}
                {ending === 'delete'
                  ? src.discard
                  : src.finish}
              </p>
            </div>
            <div className="finish-confirm-actions">
              <button className="finish-confirm-ok" onClick={handleEnd} disabled={finishing}>
                {finishing ? 'Working…' : 'Confirm'}
              </button>
              <button
                className="finish-confirm-cancel"
                onClick={() => setEnding(null)}
                disabled={finishing}
              >
                <X size={12} strokeWidth={2} />
              </button>
            </div>
          </div>
        )}

        <div className="overview-main">
          {/* Shown even with no repos: it carries the only way to attach one. */}
          <section className="overview-section">
            <h3 className="overview-section-title">
              Repositories
              <span className="overview-section-head-actions">
                <button className="overview-add-repo" onClick={() => setAddRepoOpen(true)}>
                  <Plus size={12} strokeWidth={2} style={{ marginRight: 4 }} />
                  Add repo
                </button>
              </span>
            </h3>
            {activeRepos.length === 0 ? (
              <p className="overview-empty-body">
                No repos yet — add one to check out a branch and start working.
              </p>
            ) : (
              <div className="overview-repos">
                {activeRepos.map((repo) => {
                  const wtIds = new Set(
                    activeWorktrees.filter((w) => w.repo_id === repo.id).map((w) => w.id),
                  );
                  return (
                    <RepoRow
                      key={repo.id}
                      repo={repo}
                      worktrees={activeWorktrees}
                      mrs={allMrs.filter((m) => wtIds.has(m.worktree_id))}
                      onOpenWorktree={openWorktree}
                    />
                  );
                })}
              </div>
            )}
          </section>

          <BodyEditor
            taskId={activeTask.short_id}
            source={src}
            markdown={body}
            loading={loading}
            onSaved={reload}
          />
        </div>
      </div>
    </div>
  );
}
