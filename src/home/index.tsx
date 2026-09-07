import { useState } from 'react';
import {
  Plus, RefreshCw, Search, X, Hash, Type, Boxes, Shapes, CircleDot, Flag, FolderGit2,
  GitBranch, GitFork, User, CircleCheck, PencilLine, GitPullRequest, Tag, type LucideIcon,
} from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ReviewsSection } from './ReviewsSection';
import { ActivityPanel } from './ActivityPanel';
import { highlightSegments } from './filter';
import { useFilterAutocomplete } from './useFilterAutocomplete';
import { useTabRouting, type Tab, type TabState } from './useTabRouting';

/**
 * Home: the Live, Up next and Reviews tabs under a shared filter, plus a right rail of panels.
 */

/** One icon per filter field. */
const KEY_ICON: Record<string, LucideIcon> = {
  id: Hash,
  title: Type,
  provider: Boxes,
  forge: GitFork,
  kind: Shapes,
  status: CircleDot,
  priority: Flag,
  repo: FolderGit2,
  branch: GitBranch,
  owner: User,
  author: User,
  approved: CircleCheck,
  draft: PencilLine,
  mr: GitPullRequest,
};

function HomeTab({
  id, label, state, active, onSelect,
}: {
  id: Tab;
  label: string;
  state: TabState;
  active: boolean;
  onSelect: (t: Tab) => void;
}) {
  return (
    <button className={`home-tab ${active ? 'active' : ''}`} onClick={() => onSelect(id)}>
      {label}{' '}
      {/* A dash: the query names a field this tab has no column for. */}
      <span
        className={`home-tab-count${state.applicable ? '' : ' na'}`}
        title={state.applicable ? undefined : 'This query filters on a field this tab does not have'}
      >
        {state.applicable ? state.n : '–'}
      </span>
    </button>
  );
}

export function Home() {
  const {
    draft, filter, commits, ac, acOpen, setAcOpen, acIndex, setAcIndex, acLeft, setCaret,
    inputRef, mirrorRef, measureRef, acListRef,
    syncMirror, clearFilter, commitFilter, onDraftChange, accept, onFilterKeyDown,
  } = useFilterAutocomplete();
  const {
    tab, setTab, live, upnext, reviews, noMatches, onLiveCount, onUpnextCount, onReviewsCount,
  } = useTabRouting(filter, commits);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState('');
  const setLastError = useStore((s) => s.setLastError);
  const refreshHome = useStore((s) => s.refreshHome);
  const refreshTasks = useStore((s) => s.refreshTasks);
  const refreshReviewQueue = useStore((s) => s.refreshReviewQueue);
  const [refreshing, setRefreshing] = useState(false);

  const createExplorer = async () => {
    const name = newName.trim();
    setCreating(false);
    setNewName('');
    try {
      await invoke<string>('open_explorer_session', { name: name || null });
      setTab('live');
    } catch (e) {
      setLastError(e);
    }
  };

  // Refresh for the active tab only.
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
              {/* Mirror layer: colours the recognised keys behind the transparent input. */}
              <div className="home-filter-mirror" aria-hidden="true" ref={mirrorRef}>
                {highlightSegments(draft).map((s, i) => (
                  <span key={i} className={s.kind === 'plain' ? undefined : `flt-${s.kind}`}>{s.text}</span>
                ))}
              </div>
              {/* Invisible text before the token; its width positions the suggestion list. */}
              <span className="home-filter-measure" aria-hidden="true" ref={measureRef}>
                {draft.slice(0, ac.start)}
              </span>
              <input
                className="home-filter"
                ref={inputRef}
                placeholder="provider:github priority:high -kind:explorer"
                value={draft}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => onDraftChange(e.target.value, e.target.selectionStart ?? 0)}
                onScroll={syncMirror}
                onClick={(e) => setCaret(e.currentTarget.selectionStart ?? 0)}
                onFocus={() => setAcOpen(true)}
                onBlur={() => setAcOpen(false)}
                onKeyDown={onFilterKeyDown}
              />
              {acOpen && ac.items.length > 0 && (
                <ul className="home-ac" role="listbox" ref={acListRef} style={{ left: acLeft }}>
                  {ac.items.map((s, i) => {
                    const Icon = KEY_ICON[s.field] ?? Tag;
                    return (
                      <li key={s.insert}>
                        <button
                          className={`home-ac-item ${i === acIndex ? 'active' : ''}`}
                          role="option"
                          aria-selected={i === acIndex}
                          // Keeps focus in the input; a blur closes the list first.
                          onMouseDown={(e) => e.preventDefault()}
                          onMouseEnter={() => setAcIndex(i)}
                          onClick={() => accept(s)}
                        >
                          <Icon size={14} strokeWidth={1.75} className="home-ac-icon" />
                          <span className={s.kind === 'key' ? 'home-ac-key' : 'home-ac-value'}>{s.label}</span>
                          {s.hint && <span className="home-ac-hint">{s.hint}</span>}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
            {draft && (
              <button className="home-searchbar-clear" title="Clear the filter" onClick={clearFilter}>
                <X size={13} strokeWidth={2} />
              </button>
            )}
            <button
              className="home-searchbar-go"
              title="Apply the filter (Enter)"
              onClick={commitFilter}
            >
              <Search size={14} strokeWidth={2} />
            </button>
          </div>

          {noMatches && (
            <p className="home-nomatch">No session, task or review matches this query.</p>
          )}

          <section className="home-section home-main">
            <div className="home-tabbar">
              <HomeTab id="live" label="Live" state={live} active={tab === 'live'} onSelect={setTab} />
              <HomeTab id="upnext" label="Up next" state={upnext} active={tab === 'upnext'} onSelect={setTab} />
              <HomeTab id="reviews" label="Reviews" state={reviews} active={tab === 'reviews'} onSelect={setTab} />
              <span className="home-tabbar-spring" />
              <button className="home-link" onClick={refresh} title={REFRESH[tab].title}>
                <RefreshCw size={11} strokeWidth={2.2} className={refreshing ? 'spin' : undefined} />
                refresh
              </button>
            </div>

            {/* All tabs stay mounted; only the active one shows. */}
            <div className={`home-tabpanel ${tab === 'live' ? '' : 'is-hidden'}`}>
              <LiveSection filter={filter} onCount={onLiveCount} />
            </div>
            <div className={`home-tabpanel ${tab === 'upnext' ? '' : 'is-hidden'}`}>
              <UpNextSection filter={filter} onCount={onUpnextCount} />
            </div>
            <div className={`home-tabpanel ${tab === 'reviews' ? '' : 'is-hidden'}`}>
              <ReviewsSection filter={filter} onCount={onReviewsCount} />
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
