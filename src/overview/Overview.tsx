import { useStore } from '../shared/store';
import { TaskOverview } from './TaskOverview';
import { ReviewOverview } from './ReviewOverview';

/** The session's Overview mode, dispatched by kind: a task's Notion ticket, or
 *  a review's merge request. Rendered in the SessionShell body beside the agent. */
export function Overview() {
  const activeId = useStore((s) => s.activeSessionId);
  const session = useStore((s) => (activeId ? s.sessions[activeId] : undefined));
  if (!session) return <div className="placeholder">No session.</div>;
  return session.kind === 'review'
    ? <ReviewOverview session={session} />
    : <TaskOverview session={session} />;
}
