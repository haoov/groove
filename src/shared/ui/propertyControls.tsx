import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '../ipc/invoke';
import { Check, ChevronDown, Loader2, Plus, Search, X } from 'lucide-react';
import { statusKey } from '../lib/taskStatus';
import { priorityRank } from '../lib/taskStatus';
import type { PropertySchema, PropertyValue, RelationOption } from '../ipc/ipc';

/**
 * The property controls themselves — pills, chip rows, popovers.
 *
 * Extracted from PropertyStrip so the new-task modal edits properties with the
 * SAME controls it will be edited with afterwards on the overview. Two copies of
 * this would drift, and a task filed through a different-looking editor is exactly
 * the kind of seam that makes an app feel assembled rather than designed.
 *
 * Nothing here talks to a Notion page: every control takes a value and reports a
 * new one. The overview writes each change through immediately; the modal holds
 * them until the task exists.
 */

const RELATION_DEBOUNCE_MS = 800;

export const SINGLE_KINDS = ['status', 'select', 'date', 'number'];
export const MULTI_KINDS = ['relation', 'multi_select'];

/** Hours has its own block — it's measured, not just stored (see hours.rs). */
export const isHoursProperty = (name: string, kind: string) =>
  kind === 'number' && /^(hours spent|hours|time spent)$/i.test(name);

export function hasValue(v: PropertyValue | undefined): boolean {
  if (!v) return false;
  if (v.value === null || v.value === undefined || v.value === '') return false;
  if (Array.isArray(v.value)) return v.value.length > 0;
  if (v.kind === 'checkbox') return v.value === true;
  return true;
}

/** Unit a number wears so the pill doesn't need a separate label. */
function unitFor(name: string): string {
  if (/hours?|time/i.test(name)) return 'h';
  if (/days?/i.test(name)) return 'd';
  return '';
}

/** `2026-08-12` → `12 Aug`, with the year only when it isn't this one. */
function shortDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const sameYear = d.getFullYear() === new Date().getFullYear();
  return d.toLocaleDateString(undefined, {
    day: 'numeric', month: 'short', ...(sameYear ? {} : { year: 'numeric' }),
  });
}

export type Row = { prop: PropertySchema; current?: PropertyValue };

/** Build the `Row` a control expects from a plain draft value (modal path). */
export function draftRow(prop: PropertySchema, value: unknown): Row {
  return {
    prop,
    current: { name: prop.name, kind: prop.kind, value: value ?? null, display: '' },
  };
}

/** A single-value control: a pill that opens a popover. */
export function Pill({
  row, busy, isPriority, onChange,
}: {
  row: Row;
  busy: boolean;
  isPriority: boolean;
  onChange: (v: unknown) => void;
}) {
  const { prop, current } = row;
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement | null>(null);
  useOutsideClose(box, open, () => setOpen(false));

  const raw = current?.value;
  let text: string;
  let tone = 'neutral';
  let dot = false;

  if (prop.kind === 'status') {
    text = raw ? String(raw) : '—';
    tone = `status status-${statusKey(String(raw ?? ''))}`;
    dot = true;
  } else if (prop.kind === 'select') {
    text = raw ? String(raw) : prop.name.toLowerCase();
    tone = isPriority && raw ? `prio p${priorityRank(String(raw))}` : 'neutral';
  } else if (prop.kind === 'date') {
    const iso = String(raw ?? '').slice(0, 10);
    text = iso ? `${prop.name.toLowerCase()} ${shortDate(iso)}` : prop.name.toLowerCase();
  } else {
    const unit = unitFor(prop.name);
    text = raw === null || raw === undefined
      ? prop.name.toLowerCase()
      : `${raw}${unit || ` ${prop.name.toLowerCase()}`}`;
  }

  return (
    <span className="ppop-anchor" ref={box}>
      <button className={`ppill ${tone}`} title={prop.name} onClick={() => setOpen(!open)}>
        {dot && <span className="ppill-dot" />}
        <span className="ppill-text">{text}</span>
        {busy
          ? <Loader2 size={11} className="spin" />
          : <ChevronDown size={11} strokeWidth={3} className="ppill-caret" />}
      </button>

      {open && (
        <div className="ppop">
          <div className="ppop-head">{prop.name}</div>
          {prop.kind === 'status' || prop.kind === 'select' ? (
            <OptionList
              options={prop.options.map((o) => ({ id: o, title: o }))}
              selected={raw ? [String(raw)] : []}
              onPick={(id, on) => { onChange(on ? id : null); setOpen(false); }}
              allowClear={!!raw}
            />
          ) : (
            <ValueEditor
              kind={prop.kind}
              value={raw}
              onCommit={(v) => { onChange(v); setOpen(false); }}
            />
          )}
        </div>
      )}
    </span>
  );
}

/** Date / number editing: a real input plus an explicit clear. */
export function ValueEditor({
  kind, value, onCommit,
}: {
  kind: string;
  value: unknown;
  onCommit: (v: unknown) => void;
}) {
  const initial = kind === 'date'
    ? String(value ?? '').slice(0, 10)
    : value === null || value === undefined ? '' : String(value);
  const [text, setText] = useState(initial);

  const commit = () => {
    const t = text.trim();
    if (t === '') return onCommit(null);
    if (kind === 'number') {
      const n = Number(t);
      return onCommit(Number.isFinite(n) ? n : null);
    }
    onCommit(t);
  };

  return (
    <div className="ppop-edit">
      <input
        className="ppop-input"
        type={kind === 'date' ? 'date' : 'text'}
        inputMode={kind === 'number' ? 'decimal' : undefined}
        autoFocus
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => { if (e.key === 'Enter') commit(); }}
      />
      <div className="ppop-actions">
        <button className="ppop-btn primary" onClick={commit}>Set</button>
        <button className="ppop-btn" onClick={() => onCommit(null)}>Clear</button>
      </div>
    </div>
  );
}

/** A labelled set of chips: components, tags. */
export function MultiRow({
  row, busy, onChange, onError, debounce = RELATION_DEBOUNCE_MS,
}: {
  row: Row;
  busy: boolean;
  onChange: (v: string[]) => void;
  onError: (e: string) => void;
  /** 0 reports every change straight away — right when nothing is being written yet. */
  debounce?: number;
}) {
  const { prop, current } = row;
  const [open, setOpen] = useState(false);
  const [options, setOptions] = useState<RelationOption[] | null>(
    prop.kind === 'multi_select' ? prop.options.map((o) => ({ id: o, title: o })) : null,
  );
  const selected = (current?.value as string[]) ?? [];
  const [draft, setDraft] = useState<string[]>(selected);
  const timer = useRef<number | null>(null);
  const box = useRef<HTMLSpanElement | null>(null);
  useOutsideClose(box, open, () => setOpen(false));

  useEffect(() => { setDraft(selected); }, [selected.join(',')]);
  useEffect(() => () => { if (timer.current) clearTimeout(timer.current); }, []);

  // Relation choices come from the target database, fetched once.
  useEffect(() => {
    if (options || prop.kind !== 'relation' || !prop.relation_db) return;
    invoke<RelationOption[]>('list_relation_options', { databaseId: prop.relation_db })
      .then(setOptions)
      .catch((e) => onError(String(e)));
  }, [options, prop.kind, prop.relation_db, onError]);

  /** One write per burst of ticking, not one per tick (Notion allows ~3 req/s). */
  const stage = (ids: string[]) => {
    setDraft(ids);
    if (debounce === 0) return onChange(ids);
    if (timer.current) clearTimeout(timer.current);
    timer.current = window.setTimeout(() => onChange(ids), debounce);
  };

  const titleOf = (id: string) => options?.find((o) => o.id === id)?.title ?? '…';

  return (
    <div className="props-set">
      <span className="props-set-label" title={prop.name}>{prop.name}</span>
      <span className="props-set-chips">
        {draft.map((id) => (
          <span className="pchip" key={id}>
            {titleOf(id)}
            <button className="pchip-x" title="Remove" onClick={() => stage(draft.filter((d) => d !== id))}>
              <X size={9} strokeWidth={3.5} />
            </button>
          </span>
        ))}
        <span className="ppop-anchor" ref={box}>
          <button className="padd" onClick={() => setOpen(!open)}>
            {busy ? <Loader2 size={10} className="spin" /> : <Plus size={10} strokeWidth={3} />}
            add
          </button>
          {open && (
            <div className="ppop">
              <div className="ppop-head">{prop.name}</div>
              <OptionList
                options={options ?? []}
                selected={draft}
                loading={options === null}
                onPick={(id, on) => stage(on ? [...draft, id] : draft.filter((d) => d !== id))}
              />
            </div>
          )}
        </span>
      </span>
    </div>
  );
}

/** The option list every popover shows, with search once it gets long. */
export function OptionList({
  options, selected, onPick, loading = false, allowClear = false,
}: {
  options: { id: string; title: string }[];
  selected: string[];
  onPick: (id: string, on: boolean) => void;
  loading?: boolean;
  allowClear?: boolean;
}) {
  const [query, setQuery] = useState('');
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? options.filter((o) => o.title.toLowerCase().includes(q)) : options;
  }, [options, query]);

  return (
    <>
      {options.length > 8 && (
        <div className="ppop-search">
          <Search size={11} strokeWidth={2} />
          <input autoFocus value={query} placeholder="Filter…" onChange={(e) => setQuery(e.target.value)} />
        </div>
      )}
      <div className="ppop-list">
        {loading ? (
          <p className="ppop-empty">Loading…</p>
        ) : shown.length === 0 ? (
          <p className="ppop-empty">Nothing matches.</p>
        ) : (
          shown.map((o) => {
            const on = selected.includes(o.id);
            return (
              <button
                key={o.id}
                className={`ppop-item ${on ? 'on' : ''}`}
                onClick={() => onPick(o.id, !on)}
              >
                <span className="ppop-tick">{on && <Check size={11} strokeWidth={2.5} />}</span>
                {o.title}
              </button>
            );
          })
        )}
      </div>
      {allowClear && (
        <div className="ppop-actions">
          <button className="ppop-btn" onClick={() => onPick(selected[0], false)}>Clear</button>
        </div>
      )}
    </>
  );
}

/** Reveal an empty property so it can be filled in. */
export function AddField({ fields, onPick }: { fields: string[]; onPick: (name: string) => void }) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement | null>(null);
  useOutsideClose(box, open, () => setOpen(false));

  return (
    <span className="ppop-anchor" ref={box}>
      <button className="padd rail" onClick={() => setOpen(!open)} title="Set another property">
        <Plus size={11} strokeWidth={3} />
        field
      </button>
      {open && (
        <div className="ppop">
          <div className="ppop-head">Add a property</div>
          <div className="ppop-list">
            {fields.map((f) => (
              <button key={f} className="ppop-item" onClick={() => { onPick(f); setOpen(false); }}>
                <span className="ppop-tick" />
                {f}
              </button>
            ))}
          </div>
        </div>
      )}
    </span>
  );
}

/** Close a popover on an outside click. */
export function useOutsideClose(
  ref: React.RefObject<HTMLElement | null>,
  active: boolean,
  close: () => void,
) {
  useEffect(() => {
    if (!active) return;
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    window.addEventListener('mousedown', onDown);
    return () => window.removeEventListener('mousedown', onDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);
}
