import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '../shared/ipc/invoke';
import { Loader2, Plus } from 'lucide-react';
import { useStore } from '../shared/store';
import type { TaskTime } from '../shared/ipc/ipc';

/**
 * Time as one property column in the strip: the logged total, and a "+" that
 * opens every way to log more. Logging is always explicit (see hours.rs).
 */

/** Hours are logged to the quarter — nobody means 1.37 hours. */
const ROUND_TO = 0.25;

/** Matches .time-pop's width, so the popover can be placed before it renders. */
const POP_WIDTH = 240;

const roundHours = (seconds: number) => Math.round(seconds / 3600 / ROUND_TO) * ROUND_TO;

function human(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, '0')}`;
}

export function TimeFields({
  taskId, hoursProperty, logged, onLogged,
}: {
  taskId: string;
  /** The field hours are logged to, or null when the source has none. */
  hoursProperty: string | null;
  /** Current value at the source, read by the property strip. */
  logged: string;
  onLogged: () => void;
}) {
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);
  const [time, setTime] = useState<TaskTime | null>(null);
  const [manual, setManual] = useState('');
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);
  const btn = useRef<HTMLButtonElement | null>(null);
  const pop = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);

  // Portalled to <body>: .app-main clips its own overflow at the activity rail.
  useLayoutEffect(() => {
    if (!open) { setPos(null); return; }
    const r = btn.current?.getBoundingClientRect();
    if (!r) return;
    const left = Math.min(Math.max(8, r.right - POP_WIDTH), window.innerWidth - POP_WIDTH - 8);
    setPos({ left, top: r.bottom + 4 });
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (btn.current?.contains(t) || pop.current?.contains(t)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  const refresh = () => {
    invoke<TaskTime>('get_task_time', { taskId }).then(setTime).catch(() => setTime(null));
  };

  // The tracker ticks every 30s; a minute is live enough without polling for its
  // own sake.
  useEffect(() => {
    refresh();
    const id = window.setInterval(refresh, 60_000);
    return () => clearInterval(id);
  }, [taskId]);

  const log = async (hours: number) => {
    if (!(hours > 0) || busy) return;
    setBusy(true);
    try {
      const r = await invoke<{ before: number | null; after: number | null }>('log_task_hours', {
        shortId: taskId, hours,
      });
      notify({
        kind: 'success',
        source: 'task',
        taskId,
        title: r.after !== null && hoursProperty
          ? `Logged ${hours}h — ${hoursProperty} ${r.before} → ${r.after}`
          : `Logged ${hours}h`,
      });
      setManual('');
      setOpen(false);
      refresh();
      onLogged();
    } catch (e) {
      setLastError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const unlogged = time?.unlogged_seconds ?? 0;
  const today = time?.today_seconds ?? 0;
  const suggestion = roundHours(unlogged);
  // The source's own total where there is one, the local ledger otherwise — with
  // no external field that ledger IS the record.
  const loggedLabel = hoursProperty
    ? (logged || '0')
    : roundHours(time?.logged_seconds ?? 0).toString();

  return (
    <div className="prop prop-time">
      <span className="prop-k">Time</span>
      <span className="prop-v">
        <span className="prop-v-text">{loggedLabel}h</span>

        <span className="ppop-anchor">
          <button
            ref={btn}
            className={`time-add${unlogged > 0 ? ' pending' : ''}`}
            onClick={() => setOpen(!open)}
            title={
              unlogged > 0
                ? `${human(unlogged)} tracked, not logged yet`
                : today > 0 ? 'Tracked today, all logged' : 'Log time on this task'
            }
            aria-label="Log time on this task"
          >
            {busy ? <Loader2 size={11} className="spin" /> : <Plus size={12} strokeWidth={2.5} />}
          </button>

          {open && pos && createPortal(
            <div className="ppop time-pop" ref={pop} style={{ left: pos.left, top: pos.top }}>
              <div className="ppop-head">Log time</div>
              <div className="time-menu">
                <button
                  className="time-log"
                  disabled={busy || suggestion <= 0}
                  onClick={() => log(suggestion)}
                  title={
                    suggestion <= 0
                      ? 'Nothing tracked to log'
                      : hoursProperty
                        ? `Add ${suggestion}h to ${hoursProperty}`
                        : `Record ${suggestion}h as logged`
                  }
                >
                  {suggestion > 0 ? `Log ${suggestion}h tracked` : 'Nothing tracked to log'}
                </button>
                <div className="time-quick">
                  <button className="time-btn" disabled={busy} onClick={() => log(0.5)}>+30m</button>
                  <button className="time-btn" disabled={busy} onClick={() => log(1)}>+1h</button>
                  <input
                    className="time-input"
                    placeholder="0.0"
                    aria-label="Hours to log"
                    value={manual}
                    inputMode="decimal"
                    onChange={(e) => setManual(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key !== 'Enter') return;
                      const n = Number(manual.trim());
                      if (Number.isFinite(n) && n > 0) log(n);
                    }}
                  />
                </div>
              </div>
            </div>,
            document.body,
          )}
        </span>
      </span>
    </div>
  );
}
