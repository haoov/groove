import { useCallback, useEffect, useRef, useState } from 'react';

interface Opts {
  count: number;
  /** Enter / open the row at index i. */
  onEnter: (i: number) => void;
  /** h / ArrowLeft. Return a number to move the cursor there. */
  onLeft?: (i: number) => number | void;
  /** l / ArrowRight. Return a number to move the cursor there. */
  onRight?: (i: number) => number | void;
  /** Bump (from the store) to pull DOM focus into the list. */
  focusNonce?: number;
}

/**
 * Keyboard navigation for a vertical list (file tree / changed files): a cursor
 * row driven by j/k + arrows (at the system key-repeat rate), gg/G to jump, Enter
 * to open, and optional h/l for tree expand-collapse. The owning element gets
 * `containerRef`/`onKeyDown` and marks its cursor row with the `nav-selected`
 * class (used for scroll-into-view).
 */
export function useListNav({ count, onEnter, onLeft, onRight, focusNonce }: Opts) {
  const [storedIndex, setIndexState] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const gPending = useRef(false);
  const wantFocus = useRef(false);

  const clamp = useCallback((i: number) => Math.max(0, Math.min(i, Math.max(0, count - 1))), [count]);
  const setIndex = useCallback((i: number) => setIndexState(clamp(i)), [clamp]);

  // Clamped on read, so a list that grew or shrank needs no correcting effect.
  const index = clamp(storedIndex);

  // A panel shortcut asked for focus; the list may still be loading (the file
  // tree fetches async), so just arm the request here…
  useEffect(() => {
    if (focusNonce) wantFocus.current = true;
  }, [focusNonce]);

  // …and claim it as soon as the container is actually in the DOM (this runs
  // after every render, so it catches the late mount once files have loaded).
  useEffect(() => {
    if (wantFocus.current && containerRef.current) {
      containerRef.current.focus();
      wantFocus.current = false;
    }
  });

  // Keep the cursor row visible.
  useEffect(() => {
    containerRef.current?.querySelector('.nav-selected')?.scrollIntoView({ block: 'nearest' });
  }, [index, count]);

  const onKeyDown = useCallback((e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'j': case 'ArrowDown':
        e.preventDefault(); setIndexState((i) => clamp(i + 1)); gPending.current = false; break;
      case 'k': case 'ArrowUp':
        e.preventDefault(); setIndexState((i) => clamp(i - 1)); gPending.current = false; break;
      case 'G':
        e.preventDefault(); setIndexState(clamp(count - 1)); gPending.current = false; break;
      case 'g':
        e.preventDefault();
        if (gPending.current) { setIndexState(0); gPending.current = false; } else { gPending.current = true; }
        break;
      case 'Enter':
        e.preventDefault(); onEnter(index); gPending.current = false; break;
      case 'h': case 'ArrowLeft':
        if (onLeft) { e.preventDefault(); const r = onLeft(index); if (typeof r === 'number') setIndexState(clamp(r)); }
        gPending.current = false; break;
      case 'l': case 'ArrowRight':
        if (onRight) { e.preventDefault(); const r = onRight(index); if (typeof r === 'number') setIndexState(clamp(r)); }
        gPending.current = false; break;
      default:
        gPending.current = false;
    }
  }, [index, count, clamp, onEnter, onLeft, onRight]);

  return { index, setIndex, containerRef, onKeyDown };
}
