import { useState } from 'react';
import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ActivityPanel } from './ActivityPanel';

/**
 * Home = what is locally real, not a mirror of the Notion board.
 *
 * A big tabbed panel on the left — Live (what is checked out) and Up next (what
 * to pick up) — because a busy day has many of both and one at a time is enough.
 * A shared filter narrows whichever tab is showing. A right rail holds the
 * ambient panels (Activity now). Live/Up next come from `get_home_snapshot`;
 * Activity from `get_activity_days`.
 */

type Tab = 'live' | 'upnext';
const TAB_KEY = 'wb.homeTab';
const loadTab = (): Tab => (localStorage.getItem(TAB_KEY) === 'upnext' ? 'upnext' : 'live');

export function Home() {
  const [tab, setTabState] = useState<Tab>(loadTab);
  const [filter, setFilter] = useState('');
  const [liveCount, setLiveCount] = useState(0);
  const [upnextCount, setUpnextCount] = useState(0);

  const setTab = (t: Tab) => { setTabState(t); try { localStorage.setItem(TAB_KEY, t); } catch { /* ignore */ } };

  return (
    <div className="home">
      <div className="home-layout">
        <section className="home-section home-main">
          <div className="home-tabbar">
            <button
              className={`home-tab ${tab === 'live' ? 'active' : ''}`}
              onClick={() => setTab('live')}
            >
              Live <span className="home-tab-count">{liveCount}</span>
            </button>
            <button
              className={`home-tab ${tab === 'upnext' ? 'active' : ''}`}
              onClick={() => setTab('upnext')}
            >
              Up next <span className="home-tab-count">{upnextCount}</span>
            </button>
            <span className="home-tabbar-spring" />
            <input
              className="home-filter"
              placeholder="Filter…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Escape') setFilter(''); }}
            />
          </div>

          {/* Both stay mounted (state + counts preserved); only the active shows. */}
          <div className={`home-tabpanel ${tab === 'live' ? '' : 'is-hidden'}`}>
            <LiveSection filter={filter} onCount={setLiveCount} />
          </div>
          <div className={`home-tabpanel ${tab === 'upnext' ? '' : 'is-hidden'}`}>
            <UpNextSection filter={filter} onCount={setUpnextCount} />
          </div>
        </section>

        <aside className="home-rail">
          <ActivityPanel />
        </aside>
      </div>
    </div>
  );
}
