import { useStore } from '../shared/store';
import { FilesPanel } from '../files/FilesPanel';

/** The session workspace: a sidebar panel (Files for now) + a content pane with
 *  the open file tabs. Read-only view in 2a; the editable CodeMirror editor and
 *  the terminal land in 2b, split panes in 2c. */
export function Workspace() {
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  const setActiveTab = useStore((s) => s.setActiveTab);
  const closeTab = useStore((s) => s.closeTab);

  if (!session) {
    return <div className="placeholder">Opening session…</div>;
  }
  if (session.repos.length === 0) {
    return (
      <div className="ws">
        <div className="placeholder">
          This session has no repos yet — the add-repo wizard lands in a later slice.
        </div>
      </div>
    );
  }

  const active = session.tabs.find((t) => t.id === session.activeTabId) ?? null;

  return (
    <div className="ws">
      {session.sidebar === 'files' ? (
        <FilesPanel session={session} />
      ) : (
        <aside className="sidebar"><div className="side-h">{session.sidebar}</div><div className="tempty">Lands in a later slice.</div></aside>
      )}
      <div className="pane">
        <div className="tabs">
          {session.tabs.map((t) => (
            <div
              key={t.id}
              className={`tab${t.id === session.activeTabId ? ' on' : ''}`}
              onClick={() => setActiveTab(session.id, t.id)}
            >
              <span className="fn">{t.path.split('/').pop()}</span>
              <span className="x" onClick={(e) => { e.stopPropagation(); closeTab(session.id, t.id); }}>×</span>
            </div>
          ))}
        </div>
        <div className="pane-body">
          {!active && <div className="placeholder">Open a file from the sidebar.</div>}
          {active && active.content === null && <div className="placeholder">Loading…</div>}
          {active && active.content !== null && (
            <pre className="fileview">{active.content}</pre>
          )}
        </div>
      </div>
    </div>
  );
}
