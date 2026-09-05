import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Plus, RefreshCw, Search, X, Hash, Type, Boxes, Shapes, CircleDot, Flag, FolderGit2,
  GitBranch, GitFork, User, CircleCheck, PencilLine, GitPullRequest, Tag, type LucideIcon,
} from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import { LiveSection } from './LiveSection';
import { UpNextSection } from './UpNextSection';
import { ReviewsSection } from './ReviewsSection';
import { applySuggestion, highlightSegments, suggest, type CountReport, type Suggestion } from './filter';
import { useFilterValues } from './useFilterValues';
import { isTypingCharacter } from '../shared/lib/keys';

/**
 * Home = what is locally real, not a mirror of the task queue.
 *
 * One tabbed panel — Live (checked out), Up next (queued tasks) and Reviews
 * (MRs waiting on you), each tab showing its count. A shared filter narrows the
 * active tab.
 */

/** One icon per filter field, so a line is recognisable before it is read. */
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

/** Keep this in step with `min-width` on `.home-ac`. */
const AC_MIN_WIDTH = 260;

/** What a tab last reported for a given query. */
interface TabState {
  n: number;
  /** False when the query names a field this tab has no column for. */
  applicable: boolean;
  /** The query the numbers belong to — routing waits for all three to agree. */
  forFilter: string;
}
const EMPTY_TAB: TabState = { n: 0, applicable: true, forFilter: '' };

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
  // Autocomplete: the caret drives which token is being completed.
  const [caret, setCaret] = useState(0);
  const [acOpen, setAcOpen] = useState(false);
  const [acIndex, setAcIndex] = useState(0);
  const pendingCaret = useRef<number | null>(null);
  const acListRef = useRef<HTMLUListElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [acLeft, setAcLeft] = useState(0);
  const filterValues = useFilterValues();
  const ac = useMemo(() => suggest(draft, caret, filterValues), [draft, caret, filterValues]);
  const [live, setLive] = useState<TabState>(EMPTY_TAB);
  const [upnext, setUpnext] = useState<TabState>(EMPTY_TAB);
  const [reviews, setReviews] = useState<TabState>(EMPTY_TAB);
  const [routePending, setRoutePending] = useState(false);
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

  const clearFilter = () => { setDraft(''); setFilter(''); setCaret(0); inputRef.current?.focus(); };

  // `/` jumps to the filter — scoped to Home because this listener lives and
  // dies with it. Never steal the key from someone already typing.
  useEffect(() => {
    const onSlash = (e: KeyboardEvent) => {
      if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey || isTypingCharacter(e)) return;
      const el = e.target as HTMLElement | null;
      if (el?.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el?.tagName ?? '')) return;
      e.preventDefault();
      inputRef.current?.focus();
      setAcOpen(true);
    };
    window.addEventListener('keydown', onSlash);
    return () => window.removeEventListener('keydown', onSlash);
  }, []);

  const commitFilter = () => { setFilter(draft); setAcOpen(false); setRoutePending(true); };

  const onLiveCount = useCallback<CountReport>((n, applicable, forFilter) => setLive({ n, applicable, forFilter }), []);
  const onUpnextCount = useCallback<CountReport>((n, applicable, forFilter) => setUpnext({ n, applicable, forFilter }), []);
  const onReviewsCount = useCallback<CountReport>((n, applicable, forFilter) => setReviews({ n, applicable, forFilter }), []);

  // Every tab has answered for the query on screen. Until then the counts below
  // still describe the previous query, so nothing may be concluded from them.
  const counted = live.forFilter === filter && upnext.forFilter === filter && reviews.forFilter === filter;
  const noMatches = counted && filter.trim() !== '' && live.n === 0 && upnext.n === 0 && reviews.n === 0;

  // A committed query goes to the tab that can answer it. Routing runs ONCE per
  // commit — as a standing rule it would bounce you out of any empty tab you
  // opened on purpose. Your own tab wins whenever it has results.
  useEffect(() => {
    if (!routePending || !counted) return;
    setRoutePending(false);
    const order: [Tab, TabState][] = [['live', live], ['upnext', upnext], ['reviews', reviews]];
    const current = order.find(([id]) => id === tab)?.[1];
    if (current && current.n > 0) return;
    const target = order.find(([, s]) => s.n > 0);
    if (target) setTab(target[0]);
  }, [routePending, counted, live, upnext, reviews, tab]);

  // Put the caret back after a completion is spliced in (React has re-rendered
  // by then, so the DOM value is the new one).
  useEffect(() => {
    const at = pendingCaret.current;
    if (at === null) return;
    pendingCaret.current = null;
    inputRef.current?.setSelectionRange(at, at);
    setCaret(at);
  }, [draft]);

  // Hang the list under the token being completed, following the input's own
  // scroll. Clamped so a query typed past the right edge keeps the list on screen.
  useEffect(() => {
    const field = inputRef.current;
    const width = measureRef.current?.offsetWidth ?? 0;
    if (!field) return;
    const max = Math.max(0, field.clientWidth - AC_MIN_WIDTH);
    setAcLeft(Math.min(Math.max(0, width - field.scrollLeft), max));
  }, [draft, ac.start, acOpen]);

  // Keep the highlighted line visible — the list is taller than its box.
  useEffect(() => {
    acListRef.current?.querySelector('.home-ac-item.active')?.scrollIntoView({ block: 'nearest' });
  }, [acIndex, acOpen]);

  const accept = (s: Suggestion) => {
    const { text, caret: at } = applySuggestion(draft, ac.start, ac.end, s.insert);
    setDraft(text);
    pendingCaret.current = at;
    setAcIndex(0);
    // A key still needs its value, so keep the list up; a value ends the token.
    setAcOpen(s.kind === 'key');
    inputRef.current?.focus();
  };

  const onFilterKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    const open = acOpen && ac.items.length > 0;
    switch (e.key) {
      case 'ArrowDown':
        if (!open) return;
        e.preventDefault();
        setAcIndex((i) => (i + 1) % ac.items.length);
        return;
      case 'ArrowUp':
        if (!open) return;
        e.preventDefault();
        setAcIndex((i) => (i - 1 + ac.items.length) % ac.items.length);
        return;
      case 'Tab':
        if (!open) return;
        e.preventDefault();
        accept(ac.items[acIndex]);
        return;
      case 'Enter':
        e.preventDefault();
        // Enter takes the highlighted suggestion first, and only applies the
        // query once there is nothing left to complete.
        if (open) { accept(ac.items[acIndex]); return; }
        commitFilter();
        return;
      case 'Escape':
        e.preventDefault();
        if (open) { setAcOpen(false); return; }
        clearFilter();
        return;
      default:
        // Arrow/Home/End move the caret; read it after the browser has.
        requestAnimationFrame(() => setCaret(inputRef.current?.selectionStart ?? 0));
    }
  };

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

  const Tab = ({ id, label, state }: { id: Tab; label: string; state: TabState }) => (
    <button className={`home-tab ${tab === id ? 'active' : ''}`} onClick={() => setTab(id)}>
      {label}{' '}
      {/* A dash says the query asks for a field this tab has no column for —
          different from a zero, which means it looked and found nothing. */}
      <span
        className={`home-tab-count${state.applicable ? '' : ' na'}`}
        title={state.applicable ? undefined : 'This query filters on a field this tab does not have'}
      >
        {state.applicable ? state.n : '–'}
      </span>
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
              {/* Invisible run of the text before the token: its width IS the
                  offset the suggestion list hangs from. */}
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
                onChange={(e) => { setDraft(e.target.value); setCaret(e.target.selectionStart ?? 0); setAcOpen(true); setAcIndex(0); }}
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
                          // Keep focus in the input, or blur closes the list first.
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
              <Tab id="live" label="Live" state={live} />
              <Tab id="upnext" label="Up next" state={upnext} />
              <Tab id="reviews" label="Reviews" state={reviews} />
              <span className="home-tabbar-spring" />
              <button className="home-link" onClick={refresh} title={REFRESH[tab].title}>
                <RefreshCw size={11} strokeWidth={2.2} className={refreshing ? 'spin' : undefined} />
                refresh
              </button>
            </div>

            {/* All mounted (counts + state persist); only the active shows. */}
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
      </div>
    </div>
  );
}
