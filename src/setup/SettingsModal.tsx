import { useEffect, useState } from 'react';
import { call } from '../shared/ipc/client';
import { useStore } from '../shared/store';
import { applyUiConfig, THEMES } from '../shared/lib/ui';
import type { ConfigView } from '../shared/ipc/generated';

const THEME_LABEL: Record<string, string> = { latte: 'Latte', frappe: 'Frappé', onelight: 'One Light', onedark: 'One Dark' };

/** Preferences: theme, editor font, and size. Each change writes to the config
 *  and applies live — no restart, no Save button. */
export function SettingsModal() {
  const open = useStore((s) => s.settingsOpen);
  const setOpen = useStore((s) => s.setSettingsOpen);
  const config = useStore((s) => s.config);
  const setConfig = useStore((s) => s.setConfig);
  const [fonts, setFonts] = useState<string[]>([]);

  useEffect(() => {
    if (open) call<string[]>('list_fonts').then(setFonts).catch(() => setFonts([]));
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, setOpen]);

  if (!open || !config) return null;
  const ui = config.ui;

  // Persist one field, then re-read so the live-applied config stays the source of truth.
  const apply = async (cmd: string, args: Record<string, unknown>) => {
    try {
      await call(cmd, args);
      const next = await call<ConfigView | null>('get_config');
      setConfig(next);
      applyUiConfig(next?.ui);
    } catch (e) { console.warn(`${cmd} failed`, e); }
  };

  return (
    <div className="ov-scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) setOpen(false); }}>
      <div className="settings" role="dialog" aria-modal="true">
        <div className="settings-h">
          <span>Preferences</span>
          <button className="settings-x" onClick={() => setOpen(false)}>×</button>
        </div>

        <div className="settings-row">
          <label>Theme</label>
          <div className="theme-grid">
            {THEMES.map((t) => (
              <button key={t} className={`theme-swatch ${t}${ui.theme === t ? ' on' : ''}`} onClick={() => apply('set_theme', { theme: t })}>
                <span className="sw-dots"><i /><i /><i /></span>
                {THEME_LABEL[t] ?? t}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <label htmlFor="font-family">Editor font</label>
          <input id="font-family" list="font-list" defaultValue={ui.font_family}
            onBlur={(e) => { if (e.target.value && e.target.value !== ui.font_family) apply('set_font_family', { fontFamily: e.target.value }); }} />
          <datalist id="font-list">{fonts.map((f) => <option key={f} value={f} />)}</datalist>
        </div>

        <div className="settings-row">
          <label htmlFor="font-size">Font size</label>
          <input id="font-size" type="number" min={9} max={22} value={ui.font_size}
            onChange={(e) => { const n = parseInt(e.target.value, 10); if (n >= 9 && n <= 22) apply('set_font_size', { fontSize: n }); }} />
          <span className="settings-hint">px — editor, tree, terminals</span>
        </div>
      </div>
    </div>
  );
}
