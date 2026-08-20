import { useEffect, useMemo, useRef, useState } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { applyUiConfig, THEMES } from '../shared/lib/ui';
import { openExternal } from '../shared/lib/openExternal';
import type { TaskView, ReviewMr, ConfigView } from '../shared/ipc/generated';

interface Cmd { id: string; label: string; hint?: string; run: () => void }

const THEME_LABEL: Record<string, string> = { latte: 'Latte', frappe: 'Frappé', onelight: 'One Light', onedark: 'One Dark' };

/** Cmd+K: one keyboard-first entry point — navigate, open a task or review,
 *  switch session, change theme. Tasks and reviews load when the palette opens. */
export function CommandPalette() {
  const open = useStore((s) => s.paletteOpen);
  const setOpen = useStore((s) => s.setPaletteOpen);
  const setView = useStore((s) => s.setView);
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);
  const setActiveSession = useStore((s) => s.setActiveSession);
  const setAddRepoOpen = useStore((s) => s.setAddRepoOpen);
  const setConfig = useStore((s) => s.setConfig);
  const sessions = useStore((s) => s.sessions);
  const activeId = useStore((s) => s.activeSessionId);

  const [q, setQ] = useState('');
  const [sel, setSel] = useState(0);
  const [tasks, setTasks] = useState<TaskView[]>([]);
  const [reviews, setReviews] = useState<ReviewMr[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setQ(''); setSel(0);
    inputRef.current?.focus();
    call<TaskView[]>('list_tasks').then(setTasks).catch(() => {});
    call<ReviewMr[]>('list_review_mrs').then(setReviews).catch(() => {});
  }, [open]);

  const setTheme = async (theme: string) => {
    try { await call('set_theme', { theme }); const c = await call<ConfigView | null>('get_config'); setConfig(c); applyUiConfig(c?.ui); }
    catch (e) { console.warn('set_theme failed', e); }
  };

  const commands = useMemo<Cmd[]>(() => {
    const close = (fn: () => void) => () => { fn(); setOpen(false); };
    const list: Cmd[] = [
      { id: 'home', label: 'Go to Home', run: close(() => setView('home')) },
      { id: 'settings', label: 'Open Preferences', run: close(() => setSettingsOpen(true)) },
      { id: 'explorer', label: 'New explorer session', hint: 'scratch', run: close(() => { void call('open_explorer_session', { name: null }); }) },
      ...(activeId ? [{ id: 'addrepo', label: 'Add repo to this session', run: close(() => setAddRepoOpen(true)) }] : []),
      ...THEMES.map((t) => ({ id: `theme-${t}`, label: `Theme: ${THEME_LABEL[t] ?? t}`, run: close(() => void setTheme(t)) })),
      ...Object.values(sessions).map((s) => ({
        id: `sess-${s.id}`, label: `Switch to ${s.title}`, hint: s.id,
        run: close(() => { setActiveSession(s.id); setView('session'); }),
      })),
      ...tasks.map((t) => ({
        id: `task-${t.short_id}`, label: `Open ${t.short_id} — ${t.title}`, hint: t.status,
        run: close(() => { void call('open_task', { shortId: t.short_id }); }),
      })),
      ...reviews.map((m) => ({
        id: `mr-${m.project_full}-${m.iid}`, label: `Review !${m.iid} — ${m.title}`, hint: m.project_full,
        run: close(() => {
          if (!m.local_path) { openExternal(m.web_url); return; }
          void call('open_review_session', {
            projectFull: m.project_full, iid: m.iid, title: m.title,
            sourceBranch: m.source_branch, targetBranch: m.target_branch,
            webUrl: m.web_url, localPath: m.local_path,
          });
        }),
      })),
    ];
    return list;
  }, [sessions, tasks, reviews, activeId, setOpen, setView, setSettingsOpen, setActiveSession, setAddRepoOpen]);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return commands;
    return commands.filter((c) => (c.label + ' ' + (c.hint ?? '')).toLowerCase().includes(needle));
  }, [q, commands]);

  useEffect(() => { setSel((s) => Math.min(s, Math.max(0, filtered.length - 1))); }, [filtered.length]);

  if (!open) return null;

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') { e.preventDefault(); setOpen(false); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); setSel((s) => Math.min(s + 1, filtered.length - 1)); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); setSel((s) => Math.max(s - 1, 0)); }
    else if (e.key === 'Enter') { e.preventDefault(); filtered[sel]?.run(); }
  };

  return (
    <div className="ov-scrim palette-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) setOpen(false); }}>
      <div className="palette" role="dialog" aria-modal="true">
        <input
          ref={inputRef}
          className="palette-input"
          placeholder="Type a command, task, or review…"
          value={q}
          onChange={(e) => { setQ(e.target.value); setSel(0); }}
          onKeyDown={onKey}
        />
        <div className="palette-list">
          {filtered.length === 0 && <div className="palette-empty">No matches.</div>}
          {filtered.map((c, i) => (
            <div key={c.id} className={`palette-item${i === sel ? ' on' : ''}`}
              onMouseEnter={() => setSel(i)} onClick={() => c.run()}>
              <span className="palette-label">{c.label}</span>
              {c.hint && <span className="palette-hint">{c.hint}</span>}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
