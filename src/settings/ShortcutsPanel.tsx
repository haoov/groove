import { useEffect, useState } from 'react';
import { RotateCcw } from 'lucide-react';
import { useStore } from '../shared/store';
import { COMMANDS, type CommandId } from '../shared/lib/keybindings';
import { chordFromEvent, chordLabel, isModifierOnly, isTypingCharacter } from '../shared/lib/keys';

// Command groups in display order.
const GROUP_ORDER = ['General', 'Panels', 'Navigation', 'Workspace', 'Editor'];

export function ShortcutsPanel() {
  const keymap = useStore((s) => s.keymap);
  const setBinding = useStore((s) => s.setBinding);
  const resetKeymap = useStore((s) => s.resetKeymap);
  const setCapturingKey = useStore((s) => s.setCapturingKey);

  // Command currently capturing a new chord (null = not rebinding).
  const [capturing, setCapturing] = useState<CommandId | null>(null);

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

  const grouped = GROUP_ORDER.map((g) => ({ group: g, cmds: COMMANDS.filter((c) => c.group === g) }))
    .filter((x) => x.cmds.length > 0);

  return (
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
  );
}
