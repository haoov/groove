import { useStore } from '../shared/store';
import { FilesPanel } from '../files/FilesPanel';
import { GitPanel } from '../git/GitPanel';
import { NotesPanel } from '../notes/NotesPanel';
import { CodeEditor } from '../editor/CodeEditor';
import { TerminalTab } from '../terminal/TerminalTab';
import { ChangesView } from '../git/ChangesView';
import { activeWorktree, type Tab } from '../sessions/sessions.slice';

/** The session workspace: a sidebar panel + a content pane with the open tabs.
 *  All tab bodies stay mounted (hidden when inactive) so editor state and live
 *  terminals survive tab switches. Split panes + diff land in 2c. */
export function Workspace() {
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  const setActiveTab = useStore((s) => s.setActiveTab);
  const closeTab = useStore((s) => s.closeTab);
  const openTerminalTab = useStore((s) => s.openTerminalTab);

  if (!session) return <div className="placeholder">Opening session…</div>;
  if (session.repos.length === 0) {
    return (
      <div className="ws">
        <div className="placeholder ws-norepo">
          <p>This session has no repos yet.</p>
          <button className="fr-save" onClick={() => useStore.getState().setAddRepoOpen(true)}>Add a repo</button>
        </div>
      </div>
    );
  }

  const cwd = activeWorktree(session)?.path;

  const tabLabel = (t: Tab) =>
    t.kind === 'terminal' || t.kind === 'changes' ? t.label ?? t.kind : t.path?.split('/').pop() ?? t.id;

  return (
    <div className="ws">
      {session.sidebar === 'files' ? (
        <FilesPanel session={session} />
      ) : session.sidebar === 'git' ? (
        <GitPanel session={session} />
      ) : (
        <NotesPanel session={session} />
      )}
      <div className="pane">
        <div className="tabs">
          {session.tabs.map((t) => (
            <div
              key={t.id}
              className={`tab${t.id === session.activeTabId ? ' on' : ''}`}
              onClick={() => setActiveTab(session.id, t.id)}
            >
              {t.dirty && <span className="dot" />}
              <span className="fn">{tabLabel(t)}</span>
              <span className="x" onClick={(e) => { e.stopPropagation(); closeTab(session.id, t.id); }}>×</span>
            </div>
          ))}
          <span className="add" title="New terminal" onClick={() => openTerminalTab(session.id)}>＋</span>
        </div>
        <div className="pane-body">
          {session.tabs.length === 0 && <div className="placeholder">Open a file, or start a terminal with ＋.</div>}
          {session.tabs.map((t) => (
            <div key={t.id} className="tabbody" style={{ display: t.id === session.activeTabId ? 'block' : 'none' }}>
              {t.kind === 'terminal' ? (
                <TerminalTab sessionId={session.id} tabId={t.id} taskId={session.id} cwd={cwd} />
              ) : t.kind === 'changes' ? (
                <ChangesView session={session} />
              ) : t.content === null ? (
                <div className="placeholder">Loading…</div>
              ) : (
                <CodeEditor
                  sessionId={session.id}
                  tabId={t.id}
                  repoId={t.repoId!}
                  path={t.path!}
                  worktreePath={t.worktreePath!}
                  initial={t.content ?? ''}
                />
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
