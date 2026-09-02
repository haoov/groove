import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import type { LucideIcon } from 'lucide-react';
import { rankMatches, Highlighted } from '../lib/match';

/**
 * A search field with a ranked result list.
 *
 * The list is portalled to `document.body` and positioned against the field:
 * inside a modal it would otherwise be clipped by `overflow: hidden`. It follows
 * the anchor on scroll and resize, and flips above the field when the space
 * below is too small.
 */

type Pos = { left: number; top: number; width: number; drop: 'down' | 'up' };

const GAP = 4;
const MAX_LIST = 240;

function measure(el: HTMLElement): Pos {
  const r = el.getBoundingClientRect();
  const below = window.innerHeight - r.bottom;
  const drop: 'down' | 'up' = below < MAX_LIST && r.top > below ? 'up' : 'down';
  return {
    left: r.left,
    width: r.width,
    top: drop === 'down' ? r.bottom + GAP : r.top - GAP,
    drop,
  };
}

export function Combobox<T>({
  items, toText, onPick, placeholder, disabled = false, icon: Icon, renderItem,
  emptyLabel = 'No match', value = '', onClear, autoFocus = false,
  inputClassName = '',
}: {
  items: readonly T[];
  toText: (item: T) => string;
  onPick: (item: T) => void;
  placeholder?: string;
  disabled?: boolean;
  icon?: LucideIcon;
  /** Row body. Defaults to the item's text with the match highlighted. */
  renderItem?: (item: T, ranges: [number, number][]) => ReactNode;
  emptyLabel?: string;
  /** Shown in the field when the list is closed. */
  value?: string;
  onClear?: () => void;
  autoFocus?: boolean;
  inputClassName?: string;
}) {
  const [query, setQuery] = useState('');
  const [pos, setPos] = useState<Pos | null>(null);
  const [cursor, setCursor] = useState(0);
  const field = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);

  const open = pos !== null;
  const hits = rankMatches(query, items, toText);
  const at = Math.min(cursor, Math.max(0, hits.length - 1));

  // Opening happens in an event handler, so the position is never set from an
  // effect body.
  const show = useCallback(() => {
    if (disabled || !field.current) return;
    setPos(measure(field.current));
  }, [disabled]);

  const hide = useCallback(() => { setPos(null); setQuery(''); }, []);

  useEffect(() => {
    if (!open) return;
    const follow = () => { if (field.current) setPos(measure(field.current)); };
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (field.current?.contains(t) || list.current?.contains(t)) return;
      hide();
    };
    // Capture, so a scroll inside the modal body is seen too.
    window.addEventListener('scroll', follow, true);
    window.addEventListener('resize', follow);
    document.addEventListener('mousedown', onDown);
    return () => {
      window.removeEventListener('scroll', follow, true);
      window.removeEventListener('resize', follow);
      document.removeEventListener('mousedown', onDown);
    };
  }, [open, hide]);

  /**
   * Keep the cursor row visible. Scrolls the list itself rather than calling
   * scrollIntoView: that can scroll the modal body behind the portal, which the
   * follow-on-scroll handler would then answer by repositioning the list.
   */
  useEffect(() => {
    const box = list.current;
    if (!open || !box || hits.length === 0) return;
    const row = box.children[at] as HTMLElement | undefined;
    if (!row) return;
    const top = row.offsetTop;
    const bottom = top + row.offsetHeight;
    if (top < box.scrollTop) box.scrollTop = top;
    else if (bottom > box.scrollTop + box.clientHeight) box.scrollTop = bottom - box.clientHeight;
  }, [at, open, hits.length]);

  const pick = (item: T) => {
    onPick(item);
    hide();
    // Without the blur, focus reopens the list — over a modal that just grew.
    input.current?.blur();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) { show(); return; }
      if (!hits.length) return;
      const d = e.key === 'ArrowDown' ? 1 : -1;
      setCursor((c) => (Math.min(c, hits.length - 1) + d + hits.length) % hits.length);
    } else if (e.key === 'Enter' && open) {
      // While the list is open Enter picks; closed, it submits the modal.
      e.preventDefault();
      e.stopPropagation();
      const hit = hits[at];
      if (hit) pick(hit.item);
    } else if (e.key === 'Escape' && open) {
      // Closes the list, not the modal behind it.
      e.preventDefault();
      e.stopPropagation();
      hide();
    }
  };

  return (
    <div className="cbx" ref={field}>
      <div className={`cbx-field ${disabled ? 'disabled' : ''}`}>
        {Icon && <Icon size={13} strokeWidth={1.75} className="cbx-icon" />}
        <input
          ref={input}
          className={`cbx-input ${inputClassName}`}
          placeholder={placeholder}
          disabled={disabled}
          autoFocus={autoFocus}
          value={open ? query : value}
          onChange={(e) => { setQuery(e.target.value); setCursor(0); if (!open) show(); }}
          onFocus={show}
          onKeyDown={onKeyDown}
        />
        {!!value && !open && onClear && (
          <button type="button" className="cbx-clear" onClick={onClear}>×</button>
        )}
      </div>

      {open && pos && createPortal(
        <div
          ref={list}
          className={`cbx-list ${pos.drop === 'up' ? 'up' : ''}`}
          style={{
            left: pos.left,
            width: pos.width,
            ...(pos.drop === 'down' ? { top: pos.top } : { bottom: window.innerHeight - pos.top }),
          }}
        >
          {hits.length === 0 ? (
            <p className="cbx-empty">{emptyLabel}</p>
          ) : (
            hits.map(({ item, ranges }, i) => (
              <button
                type="button"
                key={toText(item)}
                className={`cbx-item ${i === at ? 'cursor' : ''}`}
                onMouseEnter={() => setCursor(i)}
                onClick={() => pick(item)}
              >
                {renderItem
                  ? renderItem(item, ranges)
                  : <span className="cbx-name"><Highlighted text={toText(item)} ranges={ranges} /></span>}
              </button>
            ))
          )}
        </div>,
        document.body,
      )}
    </div>
  );
}
