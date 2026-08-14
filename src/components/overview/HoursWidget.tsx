import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Clock, Loader2 } from 'lucide-react';
import { useStore } from '../../store';
import type { TaskTime } from '../../types/ipc';

/**
 * Time spent: the one property the app measures itself, so it gets a block of its
 * own rather than a pill. Big readable numbers, a real bar, and a real button —
 * this is the thing you touch every day.
 *
 * The bar's filled portion is unlogged ÷ total known work: a real ratio, no
 * invented denominator. Logging is always explicit (see hours.rs) — a timer that
 * quietly wrote to a shared Notion number would produce data nobody could trust.
 */

/** Hours are logged to the quarter — nobody means 1.37 hours. */
const ROUND_TO = 0.25;

const roundHours = (seconds: number) => Math.round(seconds / 3600 / ROUND_TO) * ROUND_TO;

function human(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, '0')}`;
}

export function HoursWidget({
  taskId, notionPageId, logged, onLogged,
}: {
  taskId: string;
  notionPageId: string;
  /** Current Notion value, read by the property strip. */
  logged: string;
  onLogged: () => void;
}) {
  const notify = useStore((s) => s.notify);
  const setLastError = useStore((s) => s.setLastError);
  const [time, setTime] = useState<TaskTime | null>(null);
  const [manual, setManual] = useState('');
  const [busy, setBusy] = useState(false);

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
      const r = await invoke<{ before: number; after: number }>('log_task_hours', {
        taskId, notionPageId, hours,
      });
      notify({
        kind: 'success',
        source: 'notion',
        taskId,
        title: `Logged ${hours}h — Hours spent ${r.before} → ${r.after}`,
      });
      setManual('');
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
  const loggedSeconds = (Number(logged) || 0) * 3600;
  const fraction = unlogged > 0 ? unlogged / (loggedSeconds + unlogged) : 0;

  return (
    <div className="time">
      <span className="time-legend">
        <Clock size={11} strokeWidth={2} />
        Time
      </span>

      <div className="time-body">
        <div className="time-figures">
          <span className="time-logged">
            <strong>{logged || '0'}</strong>
            <span className="time-unit">h</span>
            <span className="time-caption">logged</span>
          </span>
          <span className="time-tracked">
            {unlogged > 0
              ? <><strong>{human(unlogged)}</strong> tracked, not logged yet</>
              : today > 0
                ? <><strong>{human(today)}</strong> tracked today, all logged</>
                : 'nothing tracked yet'}
          </span>
        </div>

        <div className="time-bar" aria-hidden>
          <span className="time-bar-fill" style={{ width: `${Math.min(100, fraction * 100)}%` }} />
        </div>

        <div className="time-actions">
          <button
            className="time-log"
            disabled={busy || suggestion <= 0}
            onClick={() => log(suggestion)}
            title={suggestion > 0 ? `Add ${suggestion}h to Hours spent in Notion` : 'Nothing tracked to log'}
          >
            {busy ? <Loader2 size={12} className="spin" /> : null}
            {suggestion > 0 ? `Log ${suggestion}h` : 'Nothing to log'}
          </button>
          <span className="time-quick">
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
          </span>
        </div>
      </div>
    </div>
  );
}
