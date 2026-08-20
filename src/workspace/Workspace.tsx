import { useStore } from '../shared/store';
import { FilesPanel } from '../files/FilesPanel';
import { CodeEditor } from '../editor/CodeEditor';
import { TerminalTab } from '../terminal/TerminalTab';
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
  if (session.repos.length === 0 && session.kind === 'task') {
    return <div className="ws"><div className="placeholder">This session has no repos yet — the add-repo wizard lands in a later slice.</div></div>;
  }

  const cwd = activeWorktree(session)?.path;

  const tabLabel = (t: Tab) => (t.kind === 'terminal' ? t.label ?? 'terminal' : t.path?.split('/').pop() ?? t.id);

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
              ) : t.content === null ? (
                <div className="placeholder">Loading…</div>
              ) : (
                <CodeEditor
                  sessionId={session.id}
                  tabId={t.id}
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
