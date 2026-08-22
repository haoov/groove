import { useEffect, useMemo, useState } from 'react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import type { ActivityDay } from '../shared/ipc/ipc';

// A heatmap of tracked work: 7 weekday columns × 7 week rows (a square), newest
// week at the bottom. All figures come from one get_activity_days call.
const WEEKS = 7;

const dayKey = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

/** 0 = none, then four filled steps by how much was tracked that day. */
function level(seconds: number): number {
  if (seconds <= 0) return 0;
  if (seconds < 1800) return 1;   // < 30m
  if (seconds < 3600) return 2;   // < 1h
  if (seconds < 7200) return 3;   // < 2h
  return 4;
}

function human(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, '0')}`;
}

type Cell = { key: string; label: string; seconds: number; future: boolean };

export function ActivityPanel() {
  const [days, setDays] = useState<ActivityDay[] | null>(null);
  const setLastError = useStore((s) => s.setLastError);

  useEffect(() => {
    invoke<ActivityDay[]>('get_activity_days').then(setDays).catch((e) => setLastError(String(e)));
  }, [setLastError]);

  const { cells, activeDays, streak, totalSeconds } = useMemo(() => {
    const byDay = new Map((days ?? []).map((d) => [d.day, d.seconds]));

    const today = new Date();
    today.setHours(0, 0, 0, 0);
    // Monday of the current week (getDay: 0=Sun … 6=Sat).
    const weekMonday = new Date(today);
    weekMonday.setDate(today.getDate() - ((today.getDay() + 6) % 7));
    const start = new Date(weekMonday);
    start.setDate(weekMonday.getDate() - (WEEKS - 1) * 7);

    const out: Cell[] = [];
    const cursor = new Date(start);
    for (let i = 0; i < WEEKS * 7; i++) {
      const key = dayKey(cursor);
      out.push({
        key,
        label: cursor.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' }),
        seconds: byDay.get(key) ?? 0,
        future: cursor.getTime() > today.getTime(),
      });
      cursor.setDate(cursor.getDate() + 1);
    }

    const active = out.filter((c) => !c.future && c.seconds > 0).length;
    const total = out.reduce((s, c) => s + c.seconds, 0);

    // Current streak: consecutive tracked days ending today (or yesterday, so a
    // day you haven't started yet doesn't read as a broken streak).
    let s = 0;
    const d = new Date(today);
    if ((byDay.get(dayKey(d)) ?? 0) === 0) d.setDate(d.getDate() - 1);
    while ((byDay.get(dayKey(d)) ?? 0) > 0) { s++; d.setDate(d.getDate() - 1); }

    return { cells: out, activeDays: active, streak: s, totalSeconds: total };
  }, [days]);

  return (
    <section className="home-section">
      <h2 className="home-heading">Activity</h2>

      <div className="activity-body">
        {days === null ? (
          <p className="home-empty">Loading…</p>
        ) : (
          <>
            <div className="activity-stats">
              <span className="activity-stat"><span className="k">Total</span><span className="v">{human(totalSeconds)}</span></span>
              <span className="activity-stat"><span className="k">Active days</span><span className="v">{activeDays}</span></span>
              <span className="activity-stat"><span className="k">Streak</span><span className="v">{streak}</span></span>
            </div>

            <div className="activity-cal">
              <div className="activity-weekdays">
                {['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'].map((d) => <span key={d}>{d}</span>)}
              </div>
              <div className="activity-grid">
                {cells.map((c) => (
                  <span
                    key={c.key}
                    className={`activity-cell lvl-${c.future ? 'x' : level(c.seconds)}`}
                    title={c.future ? undefined : `${c.label} · ${c.seconds > 0 ? human(c.seconds) : 'nothing tracked'}`}
                  />
                ))}
              </div>
            </div>

            <div className="activity-legend">
              <span>Less</span>
              <span className="activity-cell lvl-0" />
              <span className="activity-cell lvl-1" />
              <span className="activity-cell lvl-2" />
              <span className="activity-cell lvl-3" />
              <span className="activity-cell lvl-4" />
              <span>More</span>
            </div>
          </>
        )}
      </div>
    </section>
  );
}
