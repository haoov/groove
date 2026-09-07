import { useCallback, useEffect, useState } from 'react';
import { usePersisted, type Codec } from '../shared/lib/usePersisted';
import type { CountReport } from './filter';

export type Tab = 'live' | 'upnext' | 'reviews';

/** What a tab last reported for a given query. */
export interface TabState {
  n: number;
  /** False when the query names a field this tab has no column for. */
  applicable: boolean;
  /** The query the numbers belong to. */
  forFilter: string;
}
const EMPTY_TAB: TabState = { n: 0, applicable: true, forFilter: '' };

const TAB_KEY = 'wb.homeTab';
const TAB_CODEC: Codec<Tab> = {
  parse: (raw) => (raw === 'upnext' || raw === 'reviews' ? raw : 'live'),
  write: (t) => t,
};

/** The tab counts for a query, and the one routing pass each commit earns. */
export function useTabRouting(filter: string, commits: number) {
  const [tab, setTab] = usePersisted<Tab>(TAB_KEY, 'live', TAB_CODEC);
  const [live, setLive] = useState<TabState>(EMPTY_TAB);
  const [upnext, setUpnext] = useState<TabState>(EMPTY_TAB);
  const [reviews, setReviews] = useState<TabState>(EMPTY_TAB);
  const [routedFor, setRoutedFor] = useState(commits);

  const onLiveCount = useCallback<CountReport>((n, applicable, forFilter) => setLive({ n, applicable, forFilter }), []);
  const onUpnextCount = useCallback<CountReport>((n, applicable, forFilter) => setUpnext({ n, applicable, forFilter }), []);
  const onReviewsCount = useCallback<CountReport>((n, applicable, forFilter) => setReviews({ n, applicable, forFilter }), []);

  // True once every tab has answered for the current query.
  const counted = live.forFilter === filter && upnext.forFilter === filter && reviews.forFilter === filter;
  const noMatches = counted && filter.trim() !== '' && live.n === 0 && upnext.n === 0 && reviews.n === 0;

  // Route a committed query to a tab with results, once per commit; the current tab wins when it has any.
  useEffect(() => {
    if (routedFor === commits || !counted) return;
    setRoutedFor(commits);
    const order: [Tab, TabState][] = [['live', live], ['upnext', upnext], ['reviews', reviews]];
    const current = order.find(([id]) => id === tab)?.[1];
    if (current && current.n > 0) return;
    const target = order.find(([, s]) => s.n > 0);
    if (target) setTab(target[0]);
  }, [routedFor, commits, counted, live, upnext, reviews, tab, setTab]);

  return { tab, setTab, live, upnext, reviews, noMatches, onLiveCount, onUpnextCount, onReviewsCount };
}
