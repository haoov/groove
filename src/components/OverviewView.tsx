import { useSession } from '../store';
import { TaskOverview } from './overview/TaskOverview';
import { ExplorerOverview } from './overview/ExplorerOverview';
import { ReviewOverview } from './overview/ReviewOverview';

/** The main panel's "overview" surface — task metadata for task sessions, a
 *  repo list + "create task" for explorers, the MR + review actions for reviews. */
export function OverviewView() {
  const kind = useSession((s) => s.kind);
  if (kind === 'explorer') return <ExplorerOverview />;
  if (kind === 'review') return <ReviewOverview />;
  return <TaskOverview />;
}
