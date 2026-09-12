import { useEffect, useMemo, useRef, useState } from 'react';
import { applySuggestion, suggest, type Suggestion } from './filter';
import { useFilterValues } from './useFilterValues';
import { isTypingCharacter } from '../shared/lib/pure/keys';

/** Must equal `min-width` on `.home-ac`. */
const AC_MIN_WIDTH = 260;

/** The filter bar: the typed query, the caret, and the suggestion list it drives. */
export function useFilterAutocomplete() {
  // `draft` is the typed text; `filter` is what the tabs apply. Enter commits.
  const [draft, setDraft] = useState('');
  const [filter, setFilter] = useState('');
  // Counts commits, so the same query committed twice routes twice.
  const [commits, setCommits] = useState(0);
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

  // The mirror scrolls with the input.
  const syncMirror = () => {
    if (mirrorRef.current && inputRef.current) {
      mirrorRef.current.scrollLeft = inputRef.current.scrollLeft;
    }
  };
  useEffect(syncMirror, [draft]);

  const clearFilter = () => { setDraft(''); setFilter(''); setCaret(0); inputRef.current?.focus(); };
  const commitFilter = () => { setFilter(draft); setAcOpen(false); setCommits((n) => n + 1); };

  const onDraftChange = (text: string, at: number) => {
    setDraft(text);
    setCaret(at);
    setAcOpen(true);
    setAcIndex(0);
  };

  // `/` focuses the filter, except while typing elsewhere.
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

  // Restore the caret after a completion is spliced in.
  useEffect(() => {
    const at = pendingCaret.current;
    if (at === null) return;
    pendingCaret.current = null;
    inputRef.current?.setSelectionRange(at, at);
    setCaret(at);
  }, [draft]);

  // Position the list under the token being completed, clamped to the input width.
  useEffect(() => {
    const field = inputRef.current;
    const width = measureRef.current?.offsetWidth ?? 0;
    if (!field) return;
    const max = Math.max(0, field.clientWidth - AC_MIN_WIDTH);
    setAcLeft(Math.min(Math.max(0, width - field.scrollLeft), max));
  }, [draft, ac.start, acOpen]);

  // Keep the highlighted line visible.
  useEffect(() => {
    acListRef.current?.querySelector('.home-ac-item.active')?.scrollIntoView({ block: 'nearest' });
  }, [acIndex, acOpen]);

  const accept = (s: Suggestion) => {
    const { text, caret: at } = applySuggestion(draft, ac.start, ac.end, s.insert);
    setDraft(text);
    pendingCaret.current = at;
    setAcIndex(0);
    // A key keeps the list open; a value closes it.
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
        // Enter accepts the highlighted suggestion first, then applies the query.
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

  return {
    draft, filter, commits, ac, acOpen, setAcOpen, acIndex, setAcIndex, acLeft, setCaret,
    inputRef, mirrorRef, measureRef, acListRef,
    syncMirror, clearFilter, commitFilter, onDraftChange, accept, onFilterKeyDown,
  };
}
