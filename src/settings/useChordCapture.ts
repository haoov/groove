import { useEffect, useRef } from 'react';
import { useStore } from '../shared/store';
import { chordFromEvent, isModifierOnly, isTypingCharacter, type Chord } from '../shared/lib/keys';

/**
 * While `active`, takes the next real keystroke as a chord. Esc cancels.
 * Capture-phase listener; the store flag suspends the global keymap.
 */
export function useChordCapture(
  active: boolean,
  onCapture: (chord: Chord) => void,
  onCancel: () => void,
) {
  const setCapturingKey = useStore((s) => s.setCapturingKey);

  // A ref: fresh callbacks must not re-subscribe mid-capture.
  const cbs = useRef({ onCapture, onCancel });
  useEffect(() => { cbs.current = { onCapture, onCancel }; }, [onCapture, onCancel]);

  useEffect(() => {
    if (!active) return;
    setCapturingKey(true);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (isModifierOnly(e)) return;
      // A dead key or an AltGr character is not a chord.
      if (isTypingCharacter(e)) return;
      if (e.key === 'Escape') { cbs.current.onCancel(); return; }
      cbs.current.onCapture(chordFromEvent(e));
    };
    window.addEventListener('keydown', onKey, true);
    return () => { window.removeEventListener('keydown', onKey, true); setCapturingKey(false); };
  }, [active, setCapturingKey]);
}
