import { useEffect, useState } from 'react';
import { openExternal } from '../shared/lib/openExternal';
import { invoke } from '../shared/ipc/invoke';
import { providerCopy } from '../shared/lib/taskProvider';
import type { TaskSchema } from '../shared/ipc/ipc';
import { CheckCircle2, AlertTriangle, Trash2, X, RefreshCw, ExternalLink, Plus, Sparkles } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import type { Mr } from '../shared/ipc/ipc';
import { RepoRow } from './parts';
import { PropertyStrip } from './PropertyStrip';
import { BodyEditor } from './BodyEditor';
import { sendSkill } from '../shared/lib/agentSend';
import { offers } from '../shared/lib/skills';

export function TaskOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const activeRepos = useSession((s) => s.activeRepos);
  const setActiveRepoId = useSession((s) => s.setActiveRepoId);
  const setActiveWorktreeId = useSession((s) => s.setActiveWorktreeId);
  const setWorkspaceMode = useSession((s) => s.setWorkspaceMode);
  const setLastError = useStore((s) => s.setLastError);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);
  const sessionId = useSession((s) => s.id);
  const suggests = useStore((s) => s.config?.ui.suggest_actions ?? true);
  const kind = useSession((s) => s.kind);
  const hasStart = useStore((s) => offers(s.skills, kind, 'groove:start-task'));
  const [starting, setStarting] = useState(false);
  const [closing, setClosing] = useState(false);

  // Finishing runs through close-task, which checks for unlanded work first.
  const closeTask = async () => {
    setClosing(true);
    useStore.getState().requestConsoleFocus();
    try {
      await sendSkill(sessionId, 'groove:close-task');
    } catch (e) {
      setLastError(String(e));
    } finally {
      setClosing(false);
    }
  };

  // A suggestion, never an auto-send.
  const startTask = async () => {
    setStarting(true);
    useStore.getState().requestConsoleFocus(); // the proposal lands in the chat
    try {
      await sendSkill(sessionId, 'groove:start-task');
    } catch (e) {
      setLastError(String(e));
    } finally {
      setStarting(false);
    }
  };

  // Clicking a worktree scopes the editor to it and leaves the overview.
  const openWorktree = (repoId: string, worktreeId: string) => {
    setActiveRepoId(repoId);
    setActiveWorktreeId(worktreeId);
    setWorkspaceMode('code');
  };
  const [allMrs, setAllMrs] = useState<Mr[]>([]);
  const [body, setBody] = useState('');
  const [loading, setLoading] = useState(false);
  /** Deleting discards the task at its source and tears the local workspace
   *  down. Finishing goes through the agent instead, so it has no banner. */
  const [ending, setEnding] = useState<'delete' | null>(null);
  const [finishing, setFinishing] = useState(false);
  const [schema, setSchema] = useState<TaskSchema | null>(null);
  const src = providerCopy(activeTask);
  const shortId = activeTask?.short_id;
  const [reloadNonce, setReloadNonce] = useState(0);
  const reload = () => setReloadNonce((n) => n + 1);

  useEffect(() => {
    if (!shortId) return;
    setLoading(true);
    // Markdown, not raw blocks: same renderer as MR descriptions, and markdown
    // typed literally into the source (backticks, **bold**) then displays properly.
    invoke<string>('get_task_body_markdown', { shortId })
      .then(setBody)
      .catch(() => setBody(''))
      .finally(() => setLoading(false));
  }, [shortId, reloadNonce]);

  // Per task, not per mount: a schema belongs to the source the task came from.
  useEffect(() => {
    if (!shortId) return;
    invoke<TaskSchema>('get_task_schema', { shortId })
      .then(setSchema)
      .catch(() => setSchema(null));
  }, [shortId]);

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

  // No success path to handle: delete_task emits task_finished, which closes the
  // session — this component goes with it.
  const handleEnd = async () => {
    if (!activeTask || !ending) return;
    setFinishing(true);
    try {
      await invoke('delete_task', { shortId: activeTask.short_id });
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
              onClick={() => void closeTask()}
              disabled={finishing || closing}
              title="The agent checks nothing is unlanded, writes the task up, then asks to close it"
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

        {/* Deleting only; finishing is close-task. */}
        {ending && (
          <div className="finish-confirm-banner destructive">
            <div className="finish-confirm-icon">
              <AlertTriangle size={14} strokeWidth={2} />
            </div>
            <div className="finish-confirm-body">
              <strong>Delete &ldquo;{activeTask.title}&rdquo;?</strong>
              <p>
                This will remove all local worktrees and delete task data from the local
                database. {src.discard}
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
              <div className="overview-empty-body">
                No repos yet — add one to check out a branch and start working.
                {suggests && hasStart && (
                  <button className="overview-suggest" disabled={starting} onClick={() => void startTask()}>
                    <Sparkles size={12} strokeWidth={1.75} />
                    Let the agent read the task and set it up
                  </button>
                )}
              </div>
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
