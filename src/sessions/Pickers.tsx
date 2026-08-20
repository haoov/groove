import { useStore } from '../shared/store';

/** The header context bar for a session: Active Session / Repo / Worktree, plus
 *  close. Native selects — styled, keyboard-friendly, and responsive. */
export function Pickers() {
  const activeId = useStore((s) => s.activeSessionId);
  const sessions = useStore((s) => s.sessions);
  const session = activeId ? sessions[activeId] : undefined;
  const setActiveSession = useStore((s) => s.setActiveSession);
  const setActiveRepo = useStore((s) => s.setActiveRepo);
  const setActiveWorktree = useStore((s) => s.setActiveWorktree);
  const closeSession = useStore((s) => s.closeSession);

  if (!session) return null;
  const repoWts = session.worktrees.filter((w) => w.repo_id === session.activeRepoId);

  return (
    <div className="pickers">
      <label className="pick">
        <span className="k">session</span>
        <select value={session.id} onChange={(e) => setActiveSession(e.target.value)}>
          {Object.values(sessions).map((s) => <option key={s.id} value={s.id}>{s.title}</option>)}
        </select>
      </label>
      {session.repos.length > 0 && (
        <label className="pick">
          <span className="k">repo</span>
          <select value={session.activeRepoId ?? ''} onChange={(e) => setActiveRepo(session.id, e.target.value)}>
            {session.repos.map((r) => <option key={r.id} value={r.id}>{r.project}</option>)}
          </select>
        </label>
      )}
      {repoWts.length > 0 && (
        <label className="pick">
          <span className="k">worktree</span>
          <select value={session.activeWorktreeId ?? ''} onChange={(e) => setActiveWorktree(session.id, e.target.value)}>
            {repoWts.map((w) => <option key={w.id} value={w.id}>{w.branch}</option>)}
          </select>
        </label>
      )}
      <button className="iconbtn" title="Close session" onClick={() => closeSession(session.id)}>×</button>
    </div>
  );
}
