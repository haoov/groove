import { useEffect } from 'react';
import { useStore, sessionActions } from '../../shared/store';
import { COMMANDS } from '../../shared/lib/keybindings';
import { chordMatches, isModifierOnly, isTypingCharacter } from '../../shared/lib/keys';
import { runCommand } from '../../shared/lib/commands';

/** Installs the global keydown listener in the capture phase; app shortcuts win over CodeMirror/xterm. */
export function useKeybindings() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Let typed characters through.
      if (isModifierOnly(e) || isTypingCharacter(e)) return;
      const st = useStore.getState();
      if (st.capturingKey) return; // Settings is rebinding.
      const { keymap } = st;

      const el = e.target as HTMLElement | null;
      const tag = el?.tagName;
      const inField = tag === 'INPUT' || tag === 'TEXTAREA' || !!el?.isContentEditable;

      // The file-search input owns Ctrl+J/K; stopPropagation cannot reach the capture phase.
      if (el?.dataset?.fileSearch === '1' && (e.ctrlKey || e.metaKey) && (e.key === 'j' || e.key === 'k')) return;

      // The repo picker's filter owns Ctrl+J/K and Ctrl+Tab.
      if (el?.dataset?.repoPicker === '1' && (e.ctrlKey || e.metaKey)) return;

      // A terminal owns Ctrl+Shift+C/V; the capture phase would run `editor.focus` first.
      if (
        (e.ctrlKey || e.metaKey) && e.shiftKey &&
        (e.code === 'KeyC' || e.code === 'KeyV') &&
        el?.closest('.pty-pane')
      ) return;

      for (const cmd of COMMANDS) {
        for (const c of keymap[cmd.id] ?? []) {
          if (!chordMatches(e, c)) continue;
          // Do not steal un-modified keys from text fields.
          if (inField && !c.alt && !c.ctrl) return;
          e.preventDefault();
          e.stopPropagation();
          runCommand(cmd.id);
          return;
        }
      }
    };
    // Mouse back/forward (M4/M5) cycle the focused pane; preventDefault stops history navigation.
    const onMouse = (e: MouseEvent) => {
      if (e.button !== 3 && e.button !== 4) return;
      const st = useStore.getState();
      if (st.view !== 'workspace') return;
      const sess = st.activeSessionId ? st.sessions[st.activeSessionId] : null;
      if (!sess || sess.panes.length < 2) return;
      e.preventDefault();
      const order = sess.panes.map((p) => p.id);
      const idx = order.indexOf(sess.activePaneId);
      const dir = e.button === 3 ? -1 : 1;
      const target = order[(idx + dir + order.length) % order.length];
      sessionActions(sess.id).focusPane(target);
    };
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('mousedown', onMouse, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('mousedown', onMouse, true);
    };
  }, []);
}
