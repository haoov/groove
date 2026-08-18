import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { X, Check, Minus, Plus, RotateCcw } from 'lucide-react';
import { useStore } from '../store';
import { THEMES, DEFAULT_FONT_SIZE, DEFAULT_THEME, type ThemeName } from '../types/ipc';
import { COMMANDS, type CommandId } from '../lib/keybindings';
import { chordFromEvent, chordLabel, isModifierOnly, isTypingCharacter } from '../lib/keys';
import type { Environment } from './FirstRun';

const FONT_MIN = 10;
const FONT_MAX = 18;

// Representative swatches for the theme picker preview (base · surface · accent · green).
const SWATCHES: Record<ThemeName, string[]> = {
  frappe:   ['#303446', '#414559', '#8caaee', '#a6d189'],
  latte:    ['#eff1f5', '#ccd0da', '#1e66f5', '#40a02b'],
  onedark:  ['#282c34', '#2c313c', '#61afef', '#98c379'],
  onelight: ['#fafafa', '#e2e4ea', '#4078f2', '#50a14f'],
};

// Command groups in display order.
const GROUP_ORDER = ['General', 'Panels', 'Navigation', 'Workspace', 'Editor'];

export function SettingsModal() {
  const open = useStore((s) => s.settingsOpen);
  const setOpen = useStore((s) => s.setSettingsOpen);
  const config = useStore((s) => s.config);
  const setTheme = useStore((s) => s.setTheme);
  const setFontSize = useStore((s) => s.setFontSize);
  const setFontFamily = useStore((s) => s.setFontFamily);
  const vimMode = useStore((s) => s.vimMode);
  const setVimMode = useStore((s) => s.setVimMode);
  const keymap = useStore((s) => s.keymap);
  const setBinding = useStore((s) => s.setBinding);
  const resetKeymap = useStore((s) => s.resetKeymap);
  const setCapturingKey = useStore((s) => s.setCapturingKey);

  const theme = config?.ui.theme ?? DEFAULT_THEME;
  const fontSize = config?.ui.font_size ?? DEFAULT_FONT_SIZE;
  const fontFamily = config?.ui.font_family ?? '';

  // Command currently capturing a new chord (null = not rebinding).
  const [capturing, setCapturing] = useState<CommandId | null>(null);

  // Real installed families, so a name that matches nothing can't be picked —
  // CSS falls through silently, which is how the font "didn't apply" at all.
  const [fonts, setFonts] = useState<string[] | null>(null);
  const [env, setEnv] = useState<Environment | null>(null);
  const loadEnv = useCallback(() => {
    invoke<Environment>('check_environment').then(setEnv).catch(() => setEnv(null));
  }, []);
  useEffect(() => { if (open) loadEnv(); }, [open, loadEnv]);
  useEffect(() => {
    if (!open || fonts) return;
    invoke<string[]>('list_fonts').then(setFonts).catch(() => setFonts([]));
  }, [open, fonts]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') setOpen(false); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, setOpen]);

  // While capturing, grab the next real keystroke as the new binding. The store
  // flag suspends the global keymap so the chord isn't also run as a command.
  useEffect(() => {
    if (!capturing) return;
    setCapturingKey(true);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (isModifierOnly(e)) return; // wait for the actual key
      // A dead key or an AltGr character is not a chord, and recording one would
      // bind a shortcut to a keystroke the user cannot press without typing.
      if (isTypingCharacter(e)) return;
      if (e.key === 'Escape') { setCapturing(null); return; } // cancel
      setBinding(capturing, [chordFromEvent(e)]);
      setCapturing(null);
    };
    window.addEventListener('keydown', onKey, true);
    return () => { window.removeEventListener('keydown', onKey, true); setCapturingKey(false); };
  }, [capturing, setBinding, setCapturingKey]);

  // Close also cancels any in-flight capture.
  const close = () => { setCapturing(null); setOpen(false); };

  if (!open) return null;

  const grouped = GROUP_ORDER.map((g) => ({ group: g, cmds: COMMANDS.filter((c) => c.group === g) }))
    .filter((x) => x.cmds.length > 0);

  return (
    <div className="settings-overlay" onClick={close}>
      <div className="settings-modal" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <span className="settings-title">Settings</span>
          <button className="settings-close" onClick={close} aria-label="Close">
            <X size={16} strokeWidth={1.75} />
          </button>
        </div>

        <div className="settings-body">
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
                {/* Empty = the theme's own stack, which is what a fresh install
                    uses: it degrades to a system mono instead of pinning a name. */}
                <option value="">Theme default</option>
                {fontFamily && !fonts.includes(fontFamily) && (
                  <option value={fontFamily}>{fontFamily} (not installed)</option>
                )}
                {fonts.map((f) => <option key={f} value={f}>{f}</option>)}
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

          <section className="settings-section">
            <div className="settings-section-title">Editor</div>
            <label className="settings-toggle-row">
              <span className="settings-toggle-text">
                <span className="settings-toggle-label">Vim mode</span>
                <span className="settings-toggle-hint">Modal editing in the code editor; h/j/k/l + / navigation in diffs.</span>
              </span>
              <button
                role="switch"
                aria-checked={vimMode}
                className={`settings-switch ${vimMode ? 'on' : ''}`}
                onClick={() => setVimMode(!vimMode)}
              >
                <span className="settings-switch-knob" />
              </button>
            </label>
          </section>

          {/* The same check the first-run screen does. Kept reachable afterwards:
              a tool uninstalled later breaks a feature with no other clue. */}
          <section className="settings-section">
            <div className="settings-section-head">
              <div className="settings-section-title" style={{ marginBottom: 0 }}>This machine</div>
              <button className="settings-reset-btn" onClick={loadEnv} title="Check again">
                <RotateCcw size={12} strokeWidth={1.75} /> Recheck
              </button>
            </div>
            {!env ? (
              <p className="settings-hint">Checking…</p>
            ) : (
              <>
                <ul className="firstrun-tools">
                  {env.tools.map((t) => (
                    <li key={t.name} className={t.path ? 'ok' : t.required ? 'missing' : 'optional'}>
                      {t.path ? <Check size={12} strokeWidth={2.5} /> : <Minus size={12} strokeWidth={2} />}
                      <code>{t.name}</code>
                      <span className="firstrun-tool-purpose">{t.purpose}</span>
                      {!t.path && <span className="firstrun-tool-tag">{t.required ? 'required' : 'optional'}</span>}
                    </li>
                  ))}
                </ul>
                <p className="settings-hint">
                  Config: <code>{env.config_path}</code>
                  {config?.git?.worktree_root && <> · Worktrees: <code>{config.git.worktree_root}</code></>}
                </p>
              </>
            )}
          </section>

          <section className="settings-section">
            <div className="settings-section-head">
              <div className="settings-section-title" style={{ marginBottom: 0 }}>Keyboard shortcuts</div>
              <button className="settings-reset-btn" onClick={resetKeymap} title="Reset all shortcuts to defaults">
                <RotateCcw size={12} strokeWidth={1.75} /> Reset
              </button>
            </div>
            <div className="settings-keys">
              {grouped.map(({ group, cmds }) => (
                <div key={group} className="settings-keys-group">
                  <div className="settings-keys-group-label">{group}</div>
                  {cmds.map((cmd) => (
                    <div key={cmd.id} className="settings-key-row">
                      <span className="settings-key-label">{cmd.label}</span>
                      <span className="settings-key-chords">
                        {capturing === cmd.id ? (
                          <span className="settings-key-capturing">Press keys… (Esc to cancel)</span>
                        ) : (
                          (keymap[cmd.id] ?? []).map((c, i) => (
                            <kbd key={i} className="settings-key-chord">{chordLabel(c)}</kbd>
                          ))
                        )}
                      </span>
                      <button
                        className="settings-key-rebind"
                        onClick={() => setCapturing(cmd.id)}
                        disabled={capturing === cmd.id}
                      >
                        Rebind
                      </button>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
