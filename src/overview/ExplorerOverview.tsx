import { Compass, Plus, Sparkles } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { sendToAgent } from '../shared/lib/agentSend';
import { actionsFor } from '../agent/prompts';
import { RepoRow } from './parts';

export function ExplorerOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);
  const sessionKey = useSession((s) => s.id);
  const setLastError = useStore((s) => s.setLastError);

  const setShowAddRepo = useStore((s) => s.setAddRepoOpen);

  // The prompt itself lives in lib/prompts (shared with the agent pill), and
  // sendToAgent starts the agent if there isn't one — waiting on its SessionStart
  // hook rather than guessing how long Claude takes to boot.
  const createTaskFromSession = async () => {
    if (!activeTask) return;
    const action = actionsFor('explorer').find((a) => a.id === 'create-task');
    if (!action) return;
    useStore.getState().requestConsoleFocus(); // surface the conversation
    try {
      await sendToAgent(sessionKey, action.build({
        shortId: activeTask.short_id,
        kind: 'explorer',
        project: activeRepos[0]?.project,
      }));
    } catch (e) {
      setLastError(String(e));
    }
  };

  if (!activeTask) return null;

  return (
    <div className="overview-view">

      <div className="overview-header">
        <div className="overview-header-top">
          <span className="overview-eyebrow">
            <span className="overview-task-id explorer-overview-id">
              <Compass size={13} strokeWidth={1.75} style={{ marginRight: 6, verticalAlign: 'middle' }} />
              {activeTask.short_id}
            </span>
            <span className="overview-badge">explorer</span>
          </span>
          <button className="finish-task-btn" onClick={createTaskFromSession} title="Draft a Notion task from this session via the agent">
            <Sparkles size={13} strokeWidth={1.75} style={{ marginRight: 6 }} />
            Create task from this session
          </button>
        </div>
        <h2 className="overview-title">{activeTask.title}</h2>
      </div>

      <div className="overview-body">
        <section className="overview-section">
          <div className="overview-section-head">
            <h3 className="overview-section-title">Repositories</h3>
            <button className="overview-add-repo" onClick={() => setShowAddRepo(true)}>
              <Plus size={12} strokeWidth={2} style={{ marginRight: 4 }} />
              Add repo
            </button>
          </div>
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
            Notion task (mirroring your template) from what you've done here. After you approve it, this session
            becomes that task — keeping the agent conversation and worktrees intact.
          </p>
        </section>
      </div>
    </div>
  );
}
