import { Bot } from 'lucide-react';

/**
 * The one button that shows the running-agents list. It sits in the agent
 * panel's head, beside the pop-out or dock-back button, docked or detached: the
 * list is the panel's, so its switch stays where the panel is. The badge is the
 * number of agents waiting on the user.
 */
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
