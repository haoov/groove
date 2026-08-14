import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CheckCircle2, AlertTriangle, X } from 'lucide-react';
import { useStore, useSession } from '../../store';
import type { Mr } from '../../types/ipc';
import { MrBadge, RepoRow } from './parts';
import { PropertyStrip } from './PropertyStrip';
import { HoursWidget } from './HoursWidget';
import { BodyEditor } from './BodyEditor';

export function TaskOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const activeRepos = useSession((s) => s.activeRepos);
  const setLastError = useStore((s) => s.setLastError);
  const [allMrs, setAllMrs] = useState<Mr[]>([]);
  const [body, setBody] = useState('');
  const [loading, setLoading] = useState(false);
  const [confirmFinish, setConfirmFinish] = useState(false);
  const [finishing, setFinishing] = useState(false);
  /** Bumped after a Notion write so the panel and hours re-read the page. */
  const [hoursLogged, setHoursLogged] = useState('');
  const [reloadNonce, setReloadNonce] = useState(0);
  const reload = () => setReloadNonce((n) => n + 1);

  useEffect(() => {
    if (!activeTask) return;
    setLoading(true);
    // Markdown, not raw blocks: same renderer as MR descriptions, and markdown
    // typed literally into Notion (backticks, **bold**) then displays properly.
    invoke<string>('get_task_body_markdown', { notionPageId: activeTask.notion_page_id })
      .then(setBody)
      .catch(() => setBody(''))
      .finally(() => setLoading(false));
  }, [activeTask?.notion_page_id, reloadNonce]);

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

  const handleFinish = async () => {
    if (!activeTask) return;
    setFinishing(true);
    try {
      await invoke('finish_task', { shortId: activeTask.short_id });
    } catch (e) {
      setLastError(String(e));
      setFinishing(false);
      setConfirmFinish(false);
    }
  };

  if (!activeTask) return null;

  return (
    <div className="overview-view">
      {/* Header — eyebrow (id + metadata) over a display-scale title. */}
      <div className="overview-header">
        <div className="overview-header-top">
          <span className="overview-eyebrow">
            <span className="overview-task-id">{activeTask.short_id}</span>
          </span>
          <button
            className="finish-task-btn"
            onClick={() => setConfirmFinish(true)}
            disabled={finishing}
          >
            <CheckCircle2 size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
            Finish task
          </button>
        </div>
        <h2 className="overview-title">{activeTask.title}</h2>

        {/* Metadata about the task, under its name. */}
        <PropertyStrip
          key={reloadNonce}
          notionPageId={activeTask.notion_page_id}
          hours={
            <HoursWidget
              taskId={activeTask.short_id}
              notionPageId={activeTask.notion_page_id}
              logged={hoursLogged}
              onLogged={reload}
            />
          }
          onHoursValue={setHoursLogged}
        />
      </div>

      {/* Finish confirmation banner */}
      {confirmFinish && (
        <div className="finish-confirm-banner">
          <div className="finish-confirm-icon">
            <AlertTriangle size={14} strokeWidth={2} />
          </div>
          <div className="finish-confirm-body">
            <strong>Finish &ldquo;{activeTask.title}&rdquo;?</strong>
            <p>
              This will remove all local worktrees, delete task data from the local database,
              and mark the task as Done in Notion.
            </p>
          </div>
          <div className="finish-confirm-actions">
            <button
              className="finish-confirm-ok"
              onClick={handleFinish}
              disabled={finishing}
            >
              {finishing ? 'Finishing…' : 'Confirm'}
            </button>
            <button
              className="finish-confirm-cancel"
              onClick={() => setConfirmFinish(false)}
              disabled={finishing}
            >
              <X size={12} strokeWidth={2} />
            </button>
          </div>
        </div>
      )}

      <div className="overview-body">
        <BodyEditor
          taskId={activeTask.short_id}
          notionPageId={activeTask.notion_page_id}
          markdown={body}
          loading={loading}
          onSaved={reload}
        />

        {/* Repos */}
        {activeRepos.length > 0 && (
          <section className="overview-section">
            <h3 className="overview-section-title">Repositories</h3>
            <div className="overview-repos">
              {activeRepos.map((repo) => (
                <RepoRow key={repo.id} repo={repo} worktrees={activeWorktrees} />
              ))}
            </div>
          </section>
        )}

        {/* MRs */}
        <section className="overview-section">
          <h3 className="overview-section-title">Merge Requests</h3>
          {allMrs.length === 0 ? (
            <p className="overview-empty-body">No open MRs.</p>
          ) : (
            <div className="overview-mrs">
              {allMrs.map((mr) => <MrBadge key={mr.id} mr={mr} />)}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
