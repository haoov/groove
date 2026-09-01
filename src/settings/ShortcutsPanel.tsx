import { useCallback, useEffect, useMemo, useState } from 'react';
import { RotateCcw, Search, X } from 'lucide-react';
import { useStore } from '../shared/store';
import {
  COMMANDS, chordOwner, commandSpec, defaultChordsFor, isDefaultBinding, searchCommands,
  type CommandId,
} from '../shared/lib/keybindings';
import { chordLabel, type Chord } from '../shared/lib/keys';
import { useChordCapture } from './useChordCapture';

// Command groups in display order. A group missing from this list still shows,
// after these — this is the only place a shortcut can be rebound, so nothing may
// fall out of the table.
const GROUP_ORDER = ['General', 'Panels', 'Navigation', 'Workspace', 'Editor'];

const groupOrder = (): string[] => {
  const extra = COMMANDS.map((c) => c.group).filter((g) => !GROUP_ORDER.includes(g));
  return [...GROUP_ORDER, ...new Set(extra)];
};

/** A captured chord that another command already holds. */
interface Conflict {
  id: CommandId;
  chord: Chord;
  owner: CommandId;
}

const labelOf = (id: CommandId): string => commandSpec(id)?.label ?? id;

/** What "reset" would put the row back to. */
function defaultLabel(id: CommandId): string {
  const chords = defaultChordsFor(id);
  return chords.length === 0 ? 'unbound' : chords.map(chordLabel).join(' / ');
}

export function ShortcutsPanel() {
  const keymap = useStore((s) => s.keymap);
  const setBinding = useStore((s) => s.setBinding);
  const resetBinding = useStore((s) => s.resetBinding);
  const resetKeymap = useStore((s) => s.resetKeymap);

  const [query, setQuery] = useState('');
  const [capturing, setCapturing] = useState<CommandId | null>(null);
  const [conflict, setConflict] = useState<Conflict | null>(null);

  // A chord another command holds is a choice, not a silent steal: `setBinding`
  // takes it off the previous owner, so the user is asked first.
  const onCapture = useCallback((chord: Chord) => {
    const id = capturing;
    if (!id) return;
    setCapturing(null);
    const owner = chordOwner(keymap, chord, id);
    if (owner) setConflict({ id, chord, owner });
    else setBinding(id, [chord]);
  }, [capturing, keymap, setBinding]);

  const onCancel = useCallback(() => setCapturing(null), []);
  useChordCapture(capturing !== null, onCapture, onCancel);

  // Esc dismisses the prompt. On the capture phase, so it does not also reach the
  // view's own Esc and leave settings entirely.
  useEffect(() => {
    if (!conflict) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      e.preventDefault();
      e.stopPropagation();
      setConflict(null);
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [conflict]);

  const startCapture = (id: CommandId) => { setConflict(null); setCapturing(id); };
  const takeChord = () => {
    if (!conflict) return;
    setBinding(conflict.id, [conflict.chord]);
    setConflict(null);
  };

  const matches = useMemo(() => searchCommands(query, keymap), [query, keymap]);
  const grouped = groupOrder()
    .map((group) => ({ group, cmds: matches.filter((c) => c.group === group) }))
    .filter((x) => x.cmds.length > 0);

  return (
    <section className="settings-section">
      <div className="settings-section-head">
        <div className="settings-section-title" style={{ marginBottom: 0 }}>Keyboard shortcuts</div>
        <button className="settings-reset-btn" onClick={resetKeymap} title="Reset all shortcuts to defaults">
          <RotateCcw size={12} strokeWidth={1.75} /> Reset all
        </button>
      </div>

      <div className="settings-key-search">
        <Search size={13} strokeWidth={1.75} />
        <input
          className="settings-key-search-input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search commands and chords…"
          aria-label="Search shortcuts"
        />
        {query && (
          <button className="settings-key-search-clear" onClick={() => setQuery('')} aria-label="Clear search">
            <X size={13} strokeWidth={2} />
          </button>
        )}
      </div>

      {matches.length === 0 ? (
        <p className="settings-hint">No command matches “{query}”.</p>
      ) : (
        <div className="settings-keys">
          {grouped.map(({ group, cmds }) => (
            <div key={group} className="settings-keys-group">
              <div className="settings-keys-group-label">{group}</div>
              {cmds.map((cmd) => {
                const chords = keymap[cmd.id] ?? [];
                const atDefault = isDefaultBinding(keymap, cmd.id);
                return (
                  <div key={cmd.id} className="settings-key-item">
                    <div className="settings-key-row">
                      <span className="settings-key-label">{cmd.label}</span>
                      <span className="settings-key-chords">
                        {capturing === cmd.id ? (
                          <span className="settings-key-capturing">Press keys… (Esc to cancel)</span>
                        ) : chords.length === 0 ? (
                          <span className="settings-key-unbound">unbound</span>
                        ) : (
                          chords.map((c, i) => (
                            <kbd key={i} className="settings-key-chord">{chordLabel(c)}</kbd>
                          ))
                        )}
                      </span>
                      <span className="settings-key-actions">
                        {!atDefault && (
                          <button
                            className="settings-key-revert"
                            onClick={() => resetBinding(cmd.id)}
                            title={`Reset to ${defaultLabel(cmd.id)}`}
                            aria-label={`Reset ${cmd.label} to ${defaultLabel(cmd.id)}`}
                          >
                            <RotateCcw size={12} strokeWidth={1.75} />
                          </button>
                        )}
                        <button
                          className="settings-key-rebind"
                          onClick={() => startCapture(cmd.id)}
                          disabled={capturing === cmd.id}
                        >
                          Rebind
                        </button>
                      </span>
                    </div>

                    {conflict?.id === cmd.id && (
                      <div className="settings-key-conflict">
                        <span>
                          <kbd className="settings-key-chord">{chordLabel(conflict.chord)}</kbd> is
                          {' '}<strong>{labelOf(conflict.owner)}</strong>.
                          {(keymap[conflict.owner] ?? []).length === 1
                            ? ' Reassigning leaves it unbound.'
                            : ' Reassigning takes it off that command.'}
                        </span>
                        <button className="settings-key-take" onClick={takeChord}>Reassign</button>
                        <button className="settings-key-rebind" onClick={() => setConflict(null)}>Cancel</button>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
