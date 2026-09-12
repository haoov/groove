import { Bot } from 'lucide-react';

/** The button that shows the running-agents list. The badge counts the agents waiting on the user. */
export function AgentsToggle({
  open, count, onClick, hint,
}: {
  open: boolean;
  count: number;
  onClick: () => void;
  /** The shortcut, for the tooltip. */
  hint?: string;
}) {
  const title = `Running agents${count > 0 ? ` — ${count} waiting on you` : ''}${hint ? ` (${hint})` : ''}`;
  return (
    <button className={`pane-close agents-toggle ${open ? 'on' : ''}`} onClick={onClick} title={title}>
      <Bot size={12} strokeWidth={2} />
      {count > 0 && <span className="agents-toggle-badge">{count}</span>}
    </button>
  );
}
