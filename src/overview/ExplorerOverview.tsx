import { Compass, Plus } from 'lucide-react';
import { useStore, useSession } from '../shared/store';
import { RepoRow } from './parts';

export function ExplorerOverview() {
  const activeTask = useSession((s) => s.activeTask);
  const activeRepos = useSession((s) => s.activeRepos);
  const activeWorktrees = useSession((s) => s.activeWorktrees);

  const setShowAddRepo = useStore((s) => s.setAddRepoOpen);

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
            When this exploration is worth tracking, the agent bar's “create task” asks the agent to draft
            a task (mirroring your template, where the source has one) from what you've done here. After you
            approve it, this session becomes that task — keeping the agent conversation and worktrees intact.
          </p>
        </section>
      </div>
    </div>
  );
}
