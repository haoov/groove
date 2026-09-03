import { useStore } from '../shared/store';

export function EditorPanel() {
  const vimMode = useStore((s) => s.vimMode);
  const setVimMode = useStore((s) => s.setVimMode);

  return (
    <section className="settings-section">
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
  );
}
