import { useEffect, useState } from 'react';
import { Check, Minus, Plus } from 'lucide-react';
import { invoke } from '../shared/ipc/invoke';
import { useStore } from '../shared/store';
import {
  THEMES, DEFAULT_FONT_SIZE, DEFAULT_THEME, FONT_MIN, FONT_MAX, type ThemeName,
} from '../shared/ipc/ipc';

// Monospace faces shipped with the app (see main.tsx @fontsource imports). They are
// always selectable even though the OS font list never reports them.
const BUNDLED_FONTS = ['Lilex', 'IBM Plex Mono'];

// Representative swatches for the theme picker preview (base · surface · accent · green).
const SWATCHES: Record<ThemeName, string[]> = {
  frappe:   ['#303446', '#414559', '#8caaee', '#a6d189'],
  latte:    ['#eff1f5', '#ccd0da', '#1e66f5', '#40a02b'],
  onedark:  ['#282c34', '#2c313c', '#61afef', '#98c379'],
  onelight: ['#fafafa', '#e2e4ea', '#4078f2', '#50a14f'],
};

export function AppearancePanel() {
  const config = useStore((s) => s.config);
  const setTheme = useStore((s) => s.setTheme);
  const setFontSize = useStore((s) => s.setFontSize);
  const setFontFamily = useStore((s) => s.setFontFamily);

  const theme = config?.ui.theme ?? DEFAULT_THEME;
  const fontSize = config?.ui.font_size ?? DEFAULT_FONT_SIZE;
  const fontFamily = config?.ui.font_family ?? '';

  // Real installed families, so a name that matches nothing can't be picked —
  // CSS falls through silently, which is how the font "didn't apply" at all.
  const [fonts, setFonts] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<string[]>('list_fonts').then(setFonts).catch(() => setFonts([]));
  }, []);

  return (
    <>
      <section className="settings-section">
        <div className="settings-section-title">Theme</div>
        <div className="settings-theme-grid">
          {THEMES.map((t) => (
            <button
              key={t.id}
              className={`settings-theme-card ${theme === t.id ? 'selected' : ''}`}
              onClick={() => setTheme(t.id)}
            >
              <span className="settings-theme-swatches">
                {SWATCHES[t.id].map((c, i) => (
                  <span key={i} className="settings-theme-swatch" style={{ background: c }} />
                ))}
              </span>
              <span className="settings-theme-label">{t.label}</span>
              {theme === t.id && <Check className="settings-theme-check" size={15} strokeWidth={2.25} />}
            </button>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-title">Font size</div>
        <div className="settings-stepper">
          <button
            className="settings-step-btn"
            disabled={fontSize <= FONT_MIN}
            onClick={() => setFontSize(Math.max(FONT_MIN, fontSize - 1))}
            aria-label="Decrease font size"
          >
            <Minus size={14} strokeWidth={2} />
          </button>
          <span className="settings-step-value">{fontSize}px</span>
          <button
            className="settings-step-btn"
            disabled={fontSize >= FONT_MAX}
            onClick={() => setFontSize(Math.min(FONT_MAX, fontSize + 1))}
            aria-label="Increase font size"
          >
            <Plus size={14} strokeWidth={2} />
          </button>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-title">Font</div>
        {fonts && fonts.length > 0 ? (
          <select
            className="settings-select"
            value={fontFamily}
            onChange={(e) => setFontFamily(e.target.value)}
          >
            {/* The configured family may not be installed (a config copied
                from another machine) — keep it selectable rather than
                silently switching to whatever sorts first. */}
            {/* Empty = the theme's own stack (Lilex first), which is what a
                fresh install uses. */}
            <option value="">Theme default (Lilex)</option>
            {/* Bundled faces ship with the app, so they are always selectable
                even though the OS font list never reports them. */}
            <optgroup label="Bundled">
              {BUNDLED_FONTS.map((f) => <option key={f} value={f}>{f}</option>)}
            </optgroup>
            {fontFamily && !fonts.includes(fontFamily) && !BUNDLED_FONTS.includes(fontFamily) && (
              <option value={fontFamily}>{fontFamily} (not installed)</option>
            )}
            <optgroup label="Installed">
              {fonts.filter((f) => !BUNDLED_FONTS.includes(f)).map((f) => <option key={f} value={f}>{f}</option>)}
            </optgroup>
          </select>
        ) : (
          <input
            className="settings-input"
            value={fontFamily}
            placeholder="Monospace family name"
            onChange={(e) => setFontFamily(e.target.value)}
          />
        )}
        <p className="settings-hint">
          Applies to the editor, the file tree and the terminals.
        </p>
      </section>
    </>
  );
}
