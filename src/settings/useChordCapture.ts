import { useEffect, useRef } from 'react';
import { useStore } from '../shared/store';
import { chordFromEvent, isModifierOnly, isTypingCharacter, type Chord } from '../shared/lib/keys';

/**
 * While `active`, take the next real keystroke as a chord. Esc cancels.
 *
 * The listener is on the capture phase and stops the event there, so the chord
 * being recorded never also runs as a command — and the store flag suspends the
 * global keymap for anything that listens earlier still.
 */
export function useChordCapture(
  active: boolean,
  onCapture: (chord: Chord) => void,
  onCancel: () => void,
) {
  const setCapturingKey = useStore((s) => s.setCapturingKey);

  // Held in a ref so a re-render with fresh callbacks does not re-subscribe and
  // flap `capturingKey` mid-capture.
  const cbs = useRef({ onCapture, onCancel });
  useEffect(() => { cbs.current = { onCapture, onCancel }; }, [onCapture, onCancel]);

  useEffect(() => {
    if (!active) return;
    setCapturingKey(true);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (isModifierOnly(e)) return; // wait for the actual key
      // A dead key or an AltGr character is not a chord, and recording one would
      // bind a shortcut to a keystroke the user cannot press without typing.
      if (isTypingCharacter(e)) return;
      if (e.key === 'Escape') { cbs.current.onCancel(); return; }
      cbs.current.onCapture(chordFromEvent(e));
    };
    window.addEventListener('keydown', onKey, true);
    return () => { window.removeEventListener('keydown', onKey, true); setCapturingKey(false); };
  }, [active, setCapturingKey]);
}
