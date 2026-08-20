import { useSession } from '../shared/store';
import { TaskOverview } from './TaskOverview';
import { ExplorerOverview } from './ExplorerOverview';
import { ReviewOverview } from './ReviewOverview';

/** The main panel's "overview" surface — task metadata for task sessions, a
 *  repo list + "create task" for explorers, the MR + review actions for reviews. */
export function OverviewView() {
  const kind = useSession((s) => s.kind);
  if (kind === 'explorer') return <ExplorerOverview />;
  if (kind === 'review') return <ReviewOverview />;
  return <TaskOverview />;
}
