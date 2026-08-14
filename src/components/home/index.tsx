import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';

/**
 * Home = what is locally real, not a mirror of the Notion board.
 *
 * It's an outline, not a dashboard: section headings and indentation carry the
 * hierarchy, colour carries meaning (red = broken, amber = attention, green =
 * clean), and nothing is boxed or badged beyond the kind/priority pills. Two
 * sections answer two questions: what is checked out (Live) and what to pick up
 * next (Up next).
 *
 * Every number comes from one `get_home_snapshot` call (src-tauri/src/home/mod.rs).
 */
export function Home() {
  return (
    <div className="home">
      <div className="home-scroll">
        <LiveSection />
        <UpNextSection />
      </div>
    </div>
  );
}
