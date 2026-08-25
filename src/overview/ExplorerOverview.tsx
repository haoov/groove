import { useEffect, useRef, useState } from 'react';
import { ChevronUp, Compass, Plus, Sparkles } from 'lucide-react';
import { SOURCE_IDS } from '../setup/sources';
import { useStore, useSession } from '../shared/store';
import { sendToAgent } from '../shared/lib/agentSend';
import { actionsFor } from '../agent/prompts';
import { RepoRow } from './parts';
import { providerCopy } from '../shared/lib/taskProvider';

export function ExplorerOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const sessionKey = useSession((s) => s.id);
  const setLastError = useStore((s) => s.setLastError);

  const setShowAddRepo = useStore((s) => s.setAddRepoOpen);

  // The "file where?" drop-up. Closes on any outside click or Escape.
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setMenuOpen(false); };
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
    };
  }, [menuOpen]);

  // The prompt itself lives in lib/prompts (shared with the agent pill), and
  // sendToAgent starts the agent if there isn't one — waiting on its SessionStart
  // hook rather than guessing how long Claude takes to boot.
  // Naming the source is only needed when there is a choice; with one set up the
  // backend infers it.
  const sources = useStore((s) => SOURCE_IDS.filter((id) => !!s.config?.[id]));

  const createTaskFromSession = async (provider?: string) => {
    if (!activeTask) return;
    const action = actionsFor('explorer').find((a) => a.id === 'create-task');
    if (!action) return;
    useStore.getState().requestConsoleFocus(); // surface the conversation
    try {
      await sendToAgent(sessionKey, action.build({
        shortId: activeTask.short_id,
        kind: 'explorer',
        project: activeRepos[0]?.project,
        provider,
      }));
    } catch (e) {
      setLastError(String(e));
    }
  };

  if (!activeTask) return null;

  return (
    <div className="overview-view">
      <div className="overview-inner">
        <header className="overview-header">
          <span className="overview-task-id explorer-overview-id">
            <Compass size={13} strokeWidth={1.75} style={{ marginRight: 6, verticalAlign: 'middle' }} />
            {activeTask.short_id}
          </span>
          <h1 className="overview-title">{activeTask.title}</h1>
          <span className="overview-badge">explorer</span>
          <span className="overview-spring" />
          {sources.length > 1 ? (
            <span className="create-task-menu" ref={menuRef}>
              <button
                className="finish-task-btn"
                onClick={() => setMenuOpen((v) => !v)}
                title="Draft a task from this session via the agent — pick where to file it"
              >
                <Sparkles size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
                Create task
                <ChevronUp size={12} strokeWidth={2} style={{ marginLeft: 6 }} />
              </button>
              {menuOpen && (
                <div className="ctx-menu create-task-menu-panel">
                  {sources.map((src) => (
                    <button
                      key={src}
                      className="ctx-menu-item"
                      onClick={() => { setMenuOpen(false); createTaskFromSession(src); }}
                    >
                      <Sparkles size={13} strokeWidth={1.75} />
                      File in {providerCopy({ provider: src }).label}
                    </button>
                  ))}
                </div>
              )}
            </span>
          ) : (
            <button
              className="finish-task-btn"
              onClick={() => createTaskFromSession()}
              title="Draft a task from this session via the agent"
            >
              <Sparkles size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
              Create task from this session
            </button>
          )}
        </header>

        <section className="overview-section">
          <h3 className="overview-section-title">
            Repositories
            <span className="overview-section-head-actions">
              <button className="overview-add-repo" onClick={() => setShowAddRepo(true)}>
                <Plus size={12} strokeWidth={2} style={{ marginRight: 4 }} />
                Add repo
              </button>
            </span>
          </h3>
          {activeRepos.length === 0 ? (
            <p className="overview-empty-body">No repos yet — add one to start browsing and editing.</p>
          ) : (
            <div className="overview-repos">
              {activeRepos.map((repo) => (
                <RepoRow key={repo.id} repo={repo} worktrees={activeWorktrees} />
              ))}
            </div>
          )}
        </section>

        <section className="overview-section">
          <h3 className="overview-section-title">Turn this into a task</h3>
          <p className="overview-empty-body">
            When this exploration is worth tracking, “Create task from this session” asks the agent to draft a
            task (mirroring your template, where the source has one) from what you've done here. After you
            approve it, this session becomes that task — keeping the agent conversation and worktrees intact.
          </p>
        </section>
      </div>
    </div>
  );
}
