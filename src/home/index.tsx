import { useState } from 'react';
import { Plus } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ReviewsSection } from './ReviewsSection';
import { ActivityPanel } from './ActivityPanel';

/**
 * Home = what is locally real, not a mirror of the Notion board.
 *
 * A big tabbed panel on the left — Live (checked out), Up next (queued tasks)
 * and Reviews (MRs waiting on you), each tab showing its count. A shared filter
 * narrows the active tab. A right rail holds the ambient panels (Activity now).
 */

type Tab = 'live' | 'upnext' | 'reviews';
const TAB_KEY = 'wb.homeTab';
const loadTab = (): Tab => {
  const t = localStorage.getItem(TAB_KEY);
  return t === 'upnext' || t === 'reviews' ? t : 'live';
};

export function Home() {
  const [tab, setTabState] = useState<Tab>(loadTab);
  const [filter, setFilter] = useState('');
  const [liveCount, setLiveCount] = useState(0);
  const [upnextCount, setUpnextCount] = useState(0);
  const [reviewsCount, setReviewsCount] = useState(0);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const setLastError = useStore((s) => s.setLastError);

  const setTab = (t: Tab) => { setTabState(t); try { localStorage.setItem(TAB_KEY, t); } catch { /* ignore */ } };

  // New explorer lives in the header — it opens a Live session, so it belongs
  // next to the tabs, not inside one tab's body.
  const createExplorer = async () => {
    const name = newName.trim();
    setCreating(false);
    setNewName('');
    try {
      await invoke<string>('open_explorer_session', { name: name || null });
      setTab('live');
    } catch (e) {
      setLastError(String(e));
    }
  };

  const Tab = ({ id, label, count }: { id: Tab; label: string; count: number }) => (
    <button className={`home-tab ${tab === id ? 'active' : ''}`} onClick={() => setTab(id)}>
      {label} <span className="home-tab-count">{count}</span>
    </button>
  );

  return (
    <div className="home">
      <div className="home-layout">
        <section className="home-section home-main">
          <div className="home-tabbar">
            <Tab id="live" label="Live" count={liveCount} />
            <Tab id="upnext" label="Up next" count={upnextCount} />
            <Tab id="reviews" label="Reviews" count={reviewsCount} />
            {creating ? (
              <span className="explorer-new-composer">
                <input
                  className="explorer-new-input"
                  autoFocus
                  placeholder="Name this explorer…"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') createExplorer();
                    if (e.key === 'Escape') { setCreating(false); setNewName(''); }
                  }}
                />
                <button className="btn-primary" onClick={createExplorer}>Create</button>
                <button className="btn-secondary" onClick={() => { setCreating(false); setNewName(''); }}>Cancel</button>
              </span>
            ) : (
              <button className="live-btn home-new-explorer" onClick={() => { setNewName(''); setCreating(true); }}>
                <Plus size={13} strokeWidth={2} />
                New explorer
              </button>
            )}
            <span className="home-tabbar-spring" />
            <input
              className="home-filter"
              placeholder="Filter…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Escape') setFilter(''); }}
            />
          </div>

          {/* All mounted (counts + state persist); only the active shows. */}
          <div className={`home-tabpanel ${tab === 'live' ? '' : 'is-hidden'}`}>
            <LiveSection filter={filter} onCount={setLiveCount} />
          </div>
          <div className={`home-tabpanel ${tab === 'upnext' ? '' : 'is-hidden'}`}>
            <UpNextSection filter={filter} onCount={setUpnextCount} />
          </div>
          <div className={`home-tabpanel ${tab === 'reviews' ? '' : 'is-hidden'}`}>
            <ReviewsSection filter={filter} onCount={setReviewsCount} />
          </div>
        </section>

        <aside className="home-rail">
          <ActivityPanel />
        </aside>
      </div>
    </div>
  );
}
