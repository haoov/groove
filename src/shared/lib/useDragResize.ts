// Pane resizing by drag: the element follows the pointer, React state moves once.

import { useCallback, useRef, useState } from 'react';

/** A drag result inside the pane's limits. */
export function clampSize(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, value));
}

/** A persisted pane size; the fallback when it is absent or under the minimum. */
export function readStoredSize(key: string, min: number, fallback: number): number {
  const saved = Number(localStorage.getItem(key));
  return Number.isFinite(saved) && saved >= min ? saved : fallback;
}

interface DragResizeOptions {
  /** 'x' drags the width, 'y' the height. Both grow as the pointer moves towards the smaller coordinate. */
  axis: 'x' | 'y';
  min: number;
  max: number;
  /** A function is read once, on mount. */
  initial: number | (() => number);
  /** localStorage key the committed size is written to, rounded. */
  storageKey?: string;
  /** Written on the element with the size, and excluded from the measured start. */
  offset?: number;
  /** Runs once, on release. */
  onCommit?: (size: number) => void;
}

/** The resized element's ref, its committed size, and the handle's `onMouseDown`. */
export function useDragResize<T extends HTMLElement>({
  axis, min, max, initial, storageKey, offset = 0, onCommit,
}: DragResizeOptions) {
  const [size, setSize] = useState(initial);
  const ref = useRef<T>(null);

  const startDrag = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const vertical = axis === 'y';
    const prop = vertical ? 'height' : 'width';
    const origin = vertical ? e.clientY : e.clientX;
    const rect = ref.current?.getBoundingClientRect();
    const start = rect ? (vertical ? rect.height : rect.width) - offset : size;
    let latest = start;

    const move = (ev: MouseEvent) => {
      const delta = origin - (vertical ? ev.clientY : ev.clientX);
      latest = clampSize(start + delta, min, max);
      if (ref.current) ref.current.style[prop] = `${latest + offset}px`;
    };
    const up = () => {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      if (storageKey) localStorage.setItem(storageKey, String(Math.round(latest)));
      setSize(latest);
      onCommit?.(latest);
    };
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
    document.body.style.cursor = vertical ? 'row-resize' : 'col-resize';
    document.body.style.userSelect = 'none';
  }, [axis, min, max, offset, storageKey, onCommit, size]);

  return { size, ref, startDrag };
}
