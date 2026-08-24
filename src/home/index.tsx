import { useEffect, useRef, useState } from 'react';
import { Plus, RefreshCw, Search, X } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ReviewsSection } from './ReviewsSection';
import { ActivityPanel } from './ActivityPanel';
import { highlightSegments } from './filter';

/**
 * Home = what is locally real, not a mirror of the task queue.
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
  // `draft` is what you are typing; `filter` is what the tabs apply. Enter
  // commits — a query is built key by key, and refiltering mid-word is noise.
  const [draft, setDraft] = useState('');
  const [filter, setFilter] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const mirrorRef = useRef<HTMLDivElement>(null);
  const [liveCount, setLiveCount] = useState(0);
  const [upnextCount, setUpnextCount] = useState(0);
  const [reviewsCount, setReviewsCount] = useState(0);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const setLastError = useStore((s) => s.setLastError);
  const refreshHome = useStore((s) => s.refreshHome);
  const refreshTasks = useStore((s) => s.refreshTasks);
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  const [refreshing, setRefreshing] = useState(false);

  const setTab = (t: Tab) => { setTabState(t); try { localStorage.setItem(TAB_KEY, t); } catch { /* ignore */ } };

  // The mirror sits under the input, so it must scroll with it.
  const syncMirror = () => {
    if (mirrorRef.current && inputRef.current) {
      mirrorRef.current.scrollLeft = inputRef.current.scrollLeft;
    }
  };
  useEffect(syncMirror, [draft]);

  const clearFilter = () => { setDraft(''); setFilter(''); inputRef.current?.focus(); };

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

  // One button, whichever tab is showing — each tab reads a different source, so
  // a shared "refresh everything" would fetch three things to update one.
  const REFRESH: Record<Tab, { run: () => Promise<void>; title: string }> = {
    live: { run: () => refreshHome(true), title: 'Refresh sessions (also re-checks CI)' },
    upnext: { run: refreshTasks, title: 'Refresh the task queue' },
    reviews: { run: refreshReviewQueue, title: 'Refresh review requests' },
  };

  const refresh = async () => {
    if (refreshing) return;
    setRefreshing(true);
    try { await REFRESH[tab].run(); } finally { setRefreshing(false); }
  };

  const Tab = ({ id, label, count }: { id: Tab; label: string; count: number }) => (
    <button className={`home-tab ${tab === id ? 'active' : ''}`} onClick={() => setTab(id)}>
      {label} <span className="home-tab-count">{count}</span>
    </button>
  );

  return (
    <div className="home">
      <div className="home-layout">
        <div className="home-main-col">
          <header className="home-pagehead">
            <h1 className="home-pagetitle">Sessions</h1>
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
          </header>

          <div className="home-searchbar">
            <div className="home-searchbar-field">
              {/* Mirror layer: colours the recognised keys behind a transparent
                  input, so the text you type is the text you see highlighted. */}
              <div className="home-filter-mirror" aria-hidden="true" ref={mirrorRef}>
                {highlightSegments(draft).map((s, i) => (
                  <span key={i} className={s.kind === 'plain' ? undefined : `flt-${s.kind}`}>{s.text}</span>
                ))}
              </div>
              <input
                className="home-filter"
                ref={inputRef}
                placeholder="provider:github priority:high -kind:explorer"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onScroll={syncMirror}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') { e.preventDefault(); setFilter(draft); }
                  if (e.key === 'Escape') { setDraft(''); setFilter(''); }
                }}
              />
            </div>
            {draft && (
              <button className="home-searchbar-clear" title="Clear the filter" onClick={clearFilter}>
                <X size={13} strokeWidth={2} />
              </button>
            )}
            <button
              className="home-searchbar-go"
              title="Apply the filter (Enter)"
              onClick={() => setFilter(draft)}
            >
              <Search size={14} strokeWidth={2} />
            </button>
          </div>

          <section className="home-section home-main">
            <div className="home-tabbar">
              <Tab id="live" label="Live" count={liveCount} />
              <Tab id="upnext" label="Up next" count={upnextCount} />
              <Tab id="reviews" label="Reviews" count={reviewsCount} />
              <span className="home-tabbar-spring" />
              <button className="home-link" onClick={refresh} title={REFRESH[tab].title}>
                <RefreshCw size={11} strokeWidth={2.2} className={refreshing ? 'spin' : undefined} />
                refresh
              </button>
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
        </div>

        <aside className="home-rail">
          <ActivityPanel />
        </aside>
      </div>
    </div>
  );
}
