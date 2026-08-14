import { useEffect, useLayoutEffect, useRef, useState } from 'react';

/**
 * A positioned popup menu: keeps itself inside the viewport, closes on Escape
 * and on any outside click. Callers render their own item buttons as children
 * and pass a `className` so the existing CSS (`.ctx-menu`, `.context-menu`, …)
 * keeps applying — this only owns positioning + dismissal.
 */
export function ContextMenu({
  x, y, onClose, className = 'ctx-menu', children,
}: {
  x: number;
  y: number;
  onClose: () => void;
  /** CSS class for the menu container (defaults to the file-tree `.ctx-menu`). */
  className?: string;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  // Keep the menu inside the viewport.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const nx = Math.min(x, window.innerWidth - r.width - 8);
    const ny = Math.min(y, window.innerHeight - r.height - 8);
    setPos({ x: Math.max(4, nx), y: Math.max(4, ny) });
  }, [x, y]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => { if (!ref.current?.contains(e.target as Node)) onClose(); };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('mousedown', onDown, true);
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('mousedown', onDown, true);
      window.removeEventListener('keydown', onKey, true);
    };
  }, [onClose]);

  return (
    <div
      className={className}
      ref={ref}
      style={{ left: pos.x, top: pos.y }}
      onContextMenu={(e) => e.preventDefault()}
    >
      {children}
    </div>
  );
}
