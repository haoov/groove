import { useCallback, useEffect, useState } from 'react';
import { X } from 'lucide-react';
import {
  AGENTS_SIDEBAR_MAX, AGENTS_SIDEBAR_MIN,
  agentLine, clampAgentsWidth, type AgentRow,
} from '../shared/lib/agents';
import { useDragResize } from '../shared/lib/useDragResize';
import { SESSION_KIND_ICON, SESSION_KIND_LABEL } from '../shared/lib/sessionKind';
import { statusKey } from '../shared/lib/taskStatus';

/** The open sessions and what each agent is doing, as a column of the agent panel. */
export function AgentsSidebar({
  rows, width, onResize, onGo, onClose,
}: {
  rows: AgentRow[];
  width: number;
  /** The drag's result, clamped; the host stores it. */
  onResize: (width: number) => void;
  onGo: (row: AgentRow) => void;
  onClose: (row: AgentRow) => void;
}) {
  const [cursor, setCursor] = useState(() => Math.max(0, rows.findIndex((r) => r.active)));
  const commit = useCallback((w: number) => onResize(clampAgentsWidth(w)), [onResize]);
  const { ref: asideRef, startDrag } = useDragResize<HTMLElement>({
    axis: 'x',
    min: AGENTS_SIDEBAR_MIN,
    max: AGENTS_SIDEBAR_MAX,
    initial: width,
    onCommit: commit,
  });

  // Clamp the cursor after a session closes.
  useEffect(() => {
    if (cursor >= rows.length) setCursor(Math.max(0, rows.length - 1));
  }, [rows.length, cursor]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (rows.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setCursor((c) => (c + 1) % rows.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setCursor((c) => (c - 1 + rows.length) % rows.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const row = rows[cursor];
      if (row) onGo(row);
    }
  };

  return (
    <>
      <div className="resize-handle" onMouseDown={startDrag} />
      <aside
        ref={asideRef}
        className="agents-sidebar"
        style={{ width }}
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        <div className="agents-list">
          {rows.length === 0 ? (
            <p className="agents-empty">No sessions open. Open a task from Home.</p>
          ) : (
            rows.map((row, i) => (
              <AgentRowView
                key={row.sessionId}
                row={row}
                cursor={i === cursor}
                onGo={() => onGo(row)}
                onClose={() => onClose(row)}
                onHover={() => setCursor(i)}
              />
            ))
          )}
        </div>
      </aside>
    </>
  );
}

function AgentRowView({
  row, cursor, onGo, onClose, onHover,
}: {
  row: AgentRow;
  cursor: boolean;
  onGo: () => void;
  onClose: () => void;
  onHover: () => void;
}) {
  const Icon = SESSION_KIND_ICON[row.kind];
  const a = row.activity;
  const waiting = a?.state === 'waiting';
  return (
    <div className={`agents-row ${row.active ? 'active' : ''} ${cursor ? 'cursor' : ''} ${waiting ? 'waiting' : ''}`}>
      <button
        className="agents-row-main"
        onClick={onGo}
        onMouseEnter={onHover}
        title={`${row.title}\n${row.idLabel ?? ''}`}
        aria-current={row.active}
      >
        <Icon size={15} strokeWidth={1.75} className="agents-row-icon" />
        <span className="agents-row-text">
          <span className="agents-row-title">{row.title}</span>
          <span className="agents-row-id">{row.idLabel ?? SESSION_KIND_LABEL[row.kind]}</span>
          {a ? (
            <span className={`agents-row-state ${a.state}`}>
              <span className={`pill-dot ${a.state}`} />
              <span className="agents-row-state-text">{agentLine(a)}</span>
            </span>
          ) : (
            <span className={`agents-row-state ${row.status ? `status-${statusKey(row.status)}` : ''}`}>
              {row.status && <span className="pill-dot" />}
              <span className="agents-row-state-text">{row.status ?? ''}</span>
            </span>
          )}
        </span>
      </button>
      <button
        className="agents-row-close"
        title="Close session"
        onClick={(e) => { e.stopPropagation(); onClose(); }}
      >
        <X size={11} strokeWidth={2.25} />
      </button>
    </div>
  );
}
