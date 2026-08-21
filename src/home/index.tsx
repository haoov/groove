import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ActivityPanel } from './ActivityPanel';

/**
 * Home = what is locally real, not a mirror of the Notion board.
 *
 * Panels tile in a responsive grid: what is checked out (Live), what to pick up
 * next (Up next), and how the work has gone (Activity — the daily time log as a
 * heatmap). Live/Up next come from one `get_home_snapshot`; Activity from
 * `get_activity_days`.
 */
export function Home() {
  return (
    <div className="home">
      <div className="home-scroll">
        <LiveSection />
        <UpNextSection />
        <ActivityPanel />
      </div>
    </div>
  );
}
